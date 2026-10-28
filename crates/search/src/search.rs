//! Iterative deepening, fail-soft negamax alpha-beta, and a captures-only
//! quiescence search.
//!
//! Phase 2 gave the baseline; Phase 3 adds the transposition table (probed
//! at every main-search node, a bound stored from the score against the
//! original window, never after an abort) and the staged move picker (TT
//! move, good captures by static exchange, killers, history quiets, bad
//! captures), searched as a principal variation search inside aspiration
//! windows, with null-move pruning and late move reductions.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::{MoveList, generate_captures};
use rusty_chess_core::types::PieceKind;

use crate::board::Board;
use crate::history::History;
use crate::killers::Killers;
use crate::movepick::MovePicker;
use crate::stat;
use crate::stats::{MAX_PLY, SearchStats, effective_branching_factor};
use crate::telemetry::{IterRecord, SCHEMA_VERSION};
use crate::tt::{Bound, TranspositionTable};

/// The default transposition table size, matching the UCI `Hash` default.
pub const DEFAULT_HASH_MB: usize = 16;

/// The score of delivering checkmate at the root. A mate at ply `p` scores
/// `MATE - p`, so shorter mates score higher.
pub const MATE: i32 = 30_000;

/// Scores at or beyond this magnitude are mates.
pub const MATE_BOUND: i32 = MATE - MAX_PLY as i32;

const INFINITY: i32 = 32_000;

/// The deepest iteration iterative deepening will attempt.
pub const MAX_DEPTH: u32 = MAX_PLY as u32 - 1;

/// How often the abort conditions are checked, in nodes.
const CHECK_EVERY: u64 = 2048;

/// Iterations below this depth are searched with the full window: the
/// previous score is not yet a useful guess.
const ASPIRATION_MIN_DEPTH: u32 = 4;

/// Null-move pruning needs at least this much depth to be worth a try.
const NULL_MOVE_MIN_DEPTH: u32 = 3;

/// Base depth reduction of the null-move search; `depth / 4` is added.
const NULL_MOVE_REDUCTION: u32 = 2;

/// Reverse futility pruning applies up to this depth.
const RFP_MAX_DEPTH: u32 = 6;

/// Reverse futility margin per ply of remaining depth, in centipawns.
const RFP_MARGIN: i32 = 80;

/// Late move pruning applies up to this depth.
const LMP_MAX_DEPTH: u32 = 4;

/// Quiet moves are pruned from move index `LMP_BASE + depth * depth` on.
const LMP_BASE: usize = 3;

/// The move index from which late move pruning skips quiet moves at `depth`.
#[inline]
pub fn lmp_threshold(depth: u32) -> usize {
    LMP_BASE + (depth * depth) as usize
}

/// Late moves are reduced from this move index on (the first ones get full depth).
const LMR_MIN_INDEX: usize = 2;

/// Late move reductions need at least this much depth.
const LMR_MIN_DEPTH: u32 = 3;

/// The reduction table, `[depth][move index]`, in plies: grows with the log
/// of both, so deep searches of late moves are cut the most.
static LMR_TABLE: std::sync::LazyLock<[[u8; 64]; 64]> = std::sync::LazyLock::new(|| {
    let mut t = [[0u8; 64]; 64];
    for (d, row) in t.iter_mut().enumerate().skip(1) {
        for (i, cell) in row.iter_mut().enumerate().skip(1) {
            let r = 0.75 + (d as f64).ln() * (i as f64).ln() / 2.25;
            *cell = r.floor().clamp(0.0, 8.0) as u8;
        }
    }
    t
});

/// The late-move reduction for a quiet move at `index` searched at `depth`.
#[inline]
pub fn lmr_reduction(depth: u32, index: usize) -> u32 {
    LMR_TABLE[depth.min(63) as usize][index.min(63)] as u32
}

/// The initial half-width of the aspiration window, in centipawns. On the
/// depth-6 bench, 25 re-searched 37 windows and cost nodes; 50 almost never
/// failed but saved little; 35 was the best of the settings tried.
const ASPIRATION_DELTA: i32 = 35;

/// Whether `score` is a mate for either side.
#[inline]
pub fn is_mate_score(score: i32) -> bool {
    score.abs() >= MATE_BOUND
}

/// Moves to mate from `score`: positive if the side to move mates, negative
/// if it is mated, `None` for a non-mate score.
pub fn mate_in(score: i32) -> Option<i32> {
    if score >= MATE_BOUND {
        Some((MATE - score + 1) / 2)
    } else if score <= -MATE_BOUND {
        Some(-((MATE + score) / 2))
    } else {
        None
    }
}

/// When to stop. Everything unset means search until the stop flag.
#[derive(Clone, Debug, Default)]
pub struct Limits {
    /// Stop after this iteration.
    pub depth: Option<u32>,
    /// Stop after about this many nodes (main plus quiescence).
    pub nodes: Option<u64>,
    /// Use exactly this long.
    pub movetime: Option<Duration>,
    /// The side to move's remaining clock time.
    pub time: Option<Duration>,
    /// The side to move's increment.
    pub inc: Option<Duration>,
    /// Moves until the next time control, if the clock is not sudden death.
    pub movestogo: Option<u32>,
    /// Search until told to stop, ignoring the clock.
    pub infinite: bool,
}

/// What a search found.
#[derive(Clone, Debug)]
pub struct SearchResult {
    /// The move to play; `Move::NULL` if there is none.
    pub best: Move,
    /// The score from the side to move's view, in centipawns or a mate score.
    pub score: i32,
    /// The last completed iteration.
    pub depth: u32,
    /// Main-search nodes.
    pub nodes: u64,
    /// Quiescence nodes.
    pub qnodes: u64,
    /// Wall-clock time.
    pub elapsed: Duration,
    /// The principal variation in UCI notation.
    pub pv: String,
    /// One record per completed iteration.
    pub iterations: Vec<IterRecord>,
}

/// A search. Holds the counters and the principal variation table; create
/// one per search thread and reuse it across searches.
pub struct Searcher {
    stop: Arc<AtomicBool>,
    tt: TranspositionTable,
    killers: Killers,
    history: History,
    stats: SearchStats,
    nodes: u64,
    qnodes: u64,
    started: Instant,
    hard_deadline: Option<Instant>,
    node_limit: Option<u64>,
    aborted: bool,
    pv: Box<[[Move; MAX_PLY]; MAX_PLY]>,
    pv_len: [usize; MAX_PLY],
}

impl Searcher {
    /// A searcher that stops when `stop` becomes true.
    pub fn new(stop: Arc<AtomicBool>) -> Searcher {
        Searcher {
            stop,
            tt: TranspositionTable::new(DEFAULT_HASH_MB),
            killers: Killers::new(),
            history: History::new(),
            stats: SearchStats::new(),
            nodes: 0,
            qnodes: 0,
            started: Instant::now(),
            hard_deadline: None,
            node_limit: None,
            aborted: false,
            pv: Box::new([[Move::NULL; MAX_PLY]; MAX_PLY]),
            pv_len: [0; MAX_PLY],
        }
    }

    /// The efficiency counters, accumulated over every search so far.
    pub fn stats(&self) -> &SearchStats {
        &self.stats
    }

    /// Zero the efficiency counters.
    pub fn reset_stats(&mut self) {
        self.stats.reset();
    }

    /// Use a different stop flag (the UCI loop shares one with its worker).
    pub fn set_stop_flag(&mut self, stop: Arc<AtomicBool>) {
        self.stop = stop;
    }

    /// Resize the transposition table (the UCI `Hash` option); this clears it.
    pub fn set_hash_mb(&mut self, mb: usize) {
        self.tt.resize(mb);
    }

    /// Clear the transposition table and the history (`ucinewgame`, and
    /// between bench positions).
    pub fn clear_hash(&mut self) {
        self.tt.clear();
        self.history.clear();
    }

    /// Per mille of the table in use by the current search.
    pub fn hashfull(&self) -> u32 {
        self.tt.hashfull()
    }

    /// Search `board` within `limits`, calling `on_iter` after each completed
    /// iteration. The board is returned to its starting state.
    pub fn search(
        &mut self,
        board: &mut Board,
        limits: &Limits,
        on_iter: &mut dyn FnMut(&IterRecord),
    ) -> SearchResult {
        self.started = Instant::now();
        self.nodes = 0;
        self.qnodes = 0;
        self.aborted = false;
        self.node_limit = limits.nodes;
        let (soft, hard) = time_plan(limits);
        self.hard_deadline = hard.map(|d| self.started + d);
        self.tt.new_search();
        self.killers.clear();
        self.history.age();
        board.set_root();
        let fen = board.position().to_fen();

        let mut root = MoveList::new();
        board.position().legal_moves(&mut root);
        if root.is_empty() {
            let score = if board.position().in_check() {
                -MATE
            } else {
                0
            };
            return SearchResult {
                best: Move::NULL,
                score,
                depth: 0,
                nodes: 0,
                qnodes: 0,
                elapsed: self.started.elapsed(),
                pv: String::new(),
                iterations: Vec::new(),
            };
        }

        let max_depth = limits.depth.unwrap_or(MAX_DEPTH).clamp(1, MAX_DEPTH);
        let mut best = root.as_slice()[0];
        let mut best_score = 0;
        let mut best_depth = 0;
        let mut pv = best.to_string();
        let mut iterations = Vec::new();
        let mut prev_total = 0u64;

        for depth in 1..=max_depth {
            let score = self.aspirate(board, depth, best_score);
            if self.aborted {
                break;
            }
            best = self.pv[0][0];
            best_score = score;
            best_depth = depth;
            pv = self.pv_string(0);
            let total = self.nodes + self.qnodes;
            let elapsed = self.started.elapsed();
            let record = IterRecord {
                v: SCHEMA_VERSION,
                fen: fen.clone(),
                depth,
                nodes: self.nodes,
                qnodes: self.qnodes,
                nps: nps(total, elapsed),
                ebf: effective_branching_factor(prev_total, total),
                fh_first: self.stats.summary().map(|s| s.fh_first),
                tt_hit: self.stats.summary().map(|s| s.tt_hit),
                score_cp: score,
                pv: pv.clone(),
            };
            prev_total = total;
            on_iter(&record);
            iterations.push(record);

            if soft.is_some_and(|s| elapsed >= s) || self.stop.load(Ordering::Relaxed) {
                break;
            }
            if self.node_limit.is_some_and(|n| total >= n) {
                break;
            }
        }

        SearchResult {
            best,
            score: best_score,
            depth: best_depth,
            nodes: self.nodes,
            qnodes: self.qnodes,
            elapsed: self.started.elapsed(),
            pv,
            iterations,
        }
    }

    /// Search `depth` in a window around `guess`, the previous iteration's
    /// score, widening and re-searching on a fail-high or fail-low. The
    /// first iterations and mate scores use the full window.
    fn aspirate(&mut self, board: &mut Board, depth: u32, guess: i32) -> i32 {
        if depth < ASPIRATION_MIN_DEPTH || is_mate_score(guess) {
            return self.negamax(board, depth, -INFINITY, INFINITY, 0, false);
        }
        let mut delta = ASPIRATION_DELTA;
        let mut alpha = (guess - delta).max(-INFINITY);
        let mut beta = (guess + delta).min(INFINITY);
        loop {
            let score = self.negamax(board, depth, alpha, beta, 0, false);
            if self.aborted {
                return score;
            }
            if score <= alpha {
                // Fail low: open downward, keep the upper bound where it was.
                beta = (alpha + beta) / 2;
                alpha = (score - delta).max(-INFINITY);
            } else if score >= beta {
                // Fail high: open upward.
                beta = (score + delta).min(INFINITY);
            } else {
                return score;
            }
            stat!(self.stats, aspiration_researches);
            delta = delta.saturating_mul(2);
            if delta >= INFINITY {
                alpha = -INFINITY;
                beta = INFINITY;
            }
        }
    }

    #[inline]
    fn should_abort(&mut self) -> bool {
        if self.aborted {
            return true;
        }
        if (self.nodes + self.qnodes) % CHECK_EVERY == 0 {
            let total = self.nodes + self.qnodes;
            if self.stop.load(Ordering::Relaxed)
                || self.node_limit.is_some_and(|n| total >= n)
                || self.hard_deadline.is_some_and(|d| Instant::now() >= d)
            {
                self.aborted = true;
            }
        }
        self.aborted
    }

    fn negamax(
        &mut self,
        board: &mut Board,
        depth: u32,
        mut alpha: i32,
        mut beta: i32,
        ply: usize,
        null_ok: bool,
    ) -> i32 {
        self.pv_len[ply] = 0;
        if ply > 0 && (board.is_repetition() || board.is_fifty_moves()) {
            return 0;
        }
        if depth == 0 {
            return self.quiescence(board, alpha, beta, ply);
        }
        if ply >= MAX_PLY - 1 {
            return board.evaluate();
        }
        self.nodes += 1;
        stat!(self.stats, nodes);
        stat!(self.stats, nodes_by_ply[ply]);
        if self.should_abort() {
            return 0;
        }

        // Mate-distance pruning: no line from here can be better than mating
        // on the next ply or worse than being mated now, so a window beyond
        // those bounds is cut to them and an empty window ends the node.
        if ply > 0 {
            alpha = alpha.max(-(MATE - ply as i32));
            beta = beta.min(MATE - ply as i32 - 1);
            if alpha >= beta {
                stat!(self.stats, mdp_cutoffs);
                return alpha;
            }
        }

        let in_check = board.position().in_check();

        // Transposition table: a cutoff when the stored depth suffices and
        // the bound agrees with the window (never at the root, which needs a
        // move), and otherwise the stored move to search first.
        let key = board.position().key();
        let alpha_orig = alpha;
        let mut tt_move = Move::NULL;
        stat!(self.stats, tt_probes);
        if let Some(hit) = self.tt.probe(key, ply) {
            stat!(self.stats, tt_hits);
            tt_move = hit.mv;
            if ply > 0 && hit.depth as u32 >= depth {
                let cutoff = match hit.bound {
                    Bound::Exact => true,
                    Bound::Lower => hit.score >= beta,
                    Bound::Upper => hit.score <= alpha,
                };
                if cutoff {
                    stat!(self.stats, tt_cutoffs);
                    return hit.score;
                }
            }
        }

        // The pruning below reasons from the static evaluation; in check it
        // is meaningless, and a mate score in the window must be searched.
        let pv_node = beta - alpha > 1;
        let prunable = ply > 0 && !in_check && !is_mate_score(beta);
        let static_eval = if prunable { board.evaluate() } else { 0 };

        // Reverse futility pruning: at shallow depth, when the static
        // evaluation beats beta by a margin that grows with the depth left,
        // the remaining search is very unlikely to bring the score back
        // under beta. Only at null-window nodes, so the principal variation
        // is always searched out.
        if prunable
            && !pv_node
            && depth <= RFP_MAX_DEPTH
            && static_eval - RFP_MARGIN * depth as i32 >= beta
        {
            stat!(self.stats, rfp_cutoffs);
            return static_eval;
        }

        // Null-move pruning: if passing the turn still fails high at a
        // reduced depth, this node almost certainly does too. Not in check,
        // not twice in a row, not without a piece to move (zugzwang), and
        // only when the static evaluation already clears beta.
        if null_ok
            && prunable
            && depth >= NULL_MOVE_MIN_DEPTH
            && board
                .position()
                .has_non_pawn_material(board.position().side_to_move())
            && static_eval >= beta
        {
            let reduction = NULL_MOVE_REDUCTION + depth / 4;
            let undo = board.make_null_move();
            let score = -self.negamax(
                board,
                depth.saturating_sub(1 + reduction),
                -beta,
                -beta + 1,
                ply + 1,
                false,
            );
            board.unmake_null_move(undo);
            if self.aborted {
                return 0;
            }
            if score >= beta {
                stat!(self.stats, null_move_cutoffs);
                // A mate found after a null move is not a real mate.
                return if is_mate_score(score) { beta } else { score };
            }
        }

        let mut picker = MovePicker::new(tt_move).with_killers(self.killers.get(ply));
        let mut best = -INFINITY;
        let mut best_move = Move::NULL;
        let mut index = 0usize;
        // Quiet moves searched at this node, for the history malus.
        let mut quiets_tried = [Move::NULL; 64];
        let mut quiets_count = 0usize;
        while let Some(m) = picker.next_with(board.position(), &self.history) {
            let quiet = !m.is_capture() && !m.is_promotion();
            let undo = board.make_move(m);
            let gives_check = board.position().in_check();
            // Late move pruning: at a shallow null-window node, once the
            // ordered moves have gone past the index that history and the
            // killers make worthwhile, the remaining quiet moves are skipped
            // without a search. Never a checking move (the forcing line may
            // be the point), and never while being mated: a defence may be
            // anywhere in the list.
            if quiet
                && !gives_check
                && prunable
                && !pv_node
                && depth <= LMP_MAX_DEPTH
                && index >= lmp_threshold(depth)
                && !is_mate_score(best)
            {
                stat!(self.stats, lmp_skips);
                board.unmake_move(m, undo);
                index += 1;
                continue;
            }
            // Check extension: a move that gives check is searched one ply
            // deeper, so forcing sequences are not cut off at the horizon.
            let new_depth = if gives_check {
                stat!(self.stats, check_extensions);
                depth
            } else {
                depth - 1
            };
            // Principal variation search: the first move gets the full
            // window; every later move a null window that only asks whether
            // it beats alpha, re-searched in full when it does (and might
            // still be below beta).
            let score = if index == 0 {
                -self.negamax(board, new_depth, -beta, -alpha, ply + 1, true)
            } else {
                // Late move reductions: a quiet move late in the list, at a
                // node not in check and not giving check, is first searched
                // shallower; only if it beats alpha anyway does it get the
                // full-depth null-window search, then the full window as in
                // PVS.
                let reduction = if quiet
                    && index >= LMR_MIN_INDEX
                    && depth >= LMR_MIN_DEPTH
                    && !in_check
                    && !gives_check
                {
                    lmr_reduction(depth, index)
                } else {
                    0
                };
                let mut probe = if reduction > 0 {
                    stat!(self.stats, lmr_reductions);
                    -self.negamax(
                        board,
                        new_depth - reduction,
                        -alpha - 1,
                        -alpha,
                        ply + 1,
                        true,
                    )
                } else {
                    -self.negamax(board, new_depth, -alpha - 1, -alpha, ply + 1, true)
                };
                if reduction > 0 && probe > alpha && !self.aborted {
                    stat!(self.stats, lmr_researches);
                    probe = -self.negamax(board, new_depth, -alpha - 1, -alpha, ply + 1, true);
                }
                if probe > alpha && probe < beta && !self.aborted {
                    -self.negamax(board, new_depth, -beta, -alpha, ply + 1, true)
                } else {
                    probe
                }
            };
            board.unmake_move(m, undo);
            if self.aborted {
                return 0; // discarded by the caller; nothing is stored
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    best_move = m;
                    self.update_pv(ply, m);
                    if alpha >= beta {
                        stat!(self.stats, fail_high_by_index[index]);
                        stat!(self.stats, cutoffs_by_stage[picker.stage() as usize]);
                        if !m.is_capture() && !m.is_promotion() {
                            self.killers.store(ply, m);
                            let side = board.position().side_to_move();
                            let bonus = History::bonus(depth);
                            self.history.update(side, m, bonus);
                            for &q in &quiets_tried[..quiets_count] {
                                self.history.update(side, q, -bonus);
                            }
                        }
                        break;
                    }
                }
            }
            if !m.is_capture() && !m.is_promotion() && quiets_count < quiets_tried.len() {
                quiets_tried[quiets_count] = m;
                quiets_count += 1;
            }
            index += 1;
        }
        if index == 0 && best == -INFINITY {
            // No legal moves at all: mate or stalemate.
            return if in_check { -(MATE - ply as i32) } else { 0 };
        }

        // The bound follows from the score against the original window.
        let bound = if best >= beta {
            Bound::Lower
        } else if best > alpha_orig {
            Bound::Exact
        } else {
            Bound::Upper
        };
        self.tt.store(key, best_move, best, depth as u8, bound, ply);
        best
    }

    fn quiescence(&mut self, board: &mut Board, mut alpha: i32, beta: i32, ply: usize) -> i32 {
        self.qnodes += 1;
        stat!(self.stats, qnodes);
        stat!(self.stats, qnodes_by_ply[ply]);
        if self.should_abort() {
            return 0;
        }
        let stand_pat = board.evaluate();
        if stand_pat >= beta || ply >= MAX_PLY - 1 {
            return stand_pat;
        }
        if stand_pat > alpha {
            alpha = stand_pat;
        }

        let mut captures = MoveList::new();
        generate_captures(board.position(), &mut captures);
        let mut scores = [0i32; rusty_chess_core::movegen::MAX_MOVES];
        score_moves(board, &captures, &mut scores);

        let mut best = stand_pat;
        for index in 0..captures.len() {
            let m = pick_next(&mut captures, &mut scores, index);
            let undo = board.make_move(m);
            let score = -self.quiescence(board, -beta, -alpha, ply + 1);
            board.unmake_move(m, undo);
            if self.aborted {
                return 0;
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }
        best
    }

    #[inline]
    fn update_pv(&mut self, ply: usize, m: Move) {
        self.pv[ply][0] = m;
        let child_len = self.pv_len[ply + 1];
        for i in 0..child_len {
            self.pv[ply][i + 1] = self.pv[ply + 1][i];
        }
        self.pv_len[ply] = child_len + 1;
    }

    fn pv_string(&self, ply: usize) -> String {
        self.pv[ply][..self.pv_len[ply]]
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Soft and hard time budgets: the search does not start a new iteration
/// after the soft budget and aborts at the hard one.
fn time_plan(limits: &Limits) -> (Option<Duration>, Option<Duration>) {
    const OVERHEAD: Duration = Duration::from_millis(10);
    if limits.infinite {
        return (None, None);
    }
    if let Some(mt) = limits.movetime {
        let t = mt.saturating_sub(OVERHEAD).max(Duration::from_millis(1));
        return (Some(t), Some(t));
    }
    if let Some(time) = limits.time {
        let inc = limits.inc.unwrap_or(Duration::ZERO);
        let moves = limits.movestogo.unwrap_or(30).max(1);
        let base = time / moves + inc.mul_f64(0.75);
        let soft = base
            .min(time / 2)
            .saturating_sub(OVERHEAD)
            .max(Duration::from_millis(1));
        let hard = (base * 3)
            .min(time / 2)
            .saturating_sub(OVERHEAD)
            .max(Duration::from_millis(1));
        return (Some(soft), Some(hard.max(soft)));
    }
    (None, None)
}

#[inline]
fn nps(nodes: u64, elapsed: Duration) -> u64 {
    let ms = elapsed.as_millis() as u64;
    (nodes * 1000).checked_div(ms).unwrap_or(0)
}

/// Quiescence ordering: most valuable victim, least valuable attacker;
/// promotions count the promoted piece.
fn score_moves(board: &Board, moves: &MoveList, scores: &mut [i32]) {
    const VALUE: [i32; 6] = [100, 320, 330, 500, 900, 0];
    let pos = board.position();
    for (i, &m) in moves.iter().enumerate() {
        let mut s = 0;
        if m.is_capture() {
            let victim = if m.is_en_passant() {
                PieceKind::Pawn
            } else {
                pos.piece_at(m.to()).map_or(PieceKind::Pawn, |p| p.kind)
            };
            let attacker = pos.piece_at(m.from()).map_or(PieceKind::Pawn, |p| p.kind);
            s = 10_000 + 10 * VALUE[victim.index()] - VALUE[attacker.index()];
        }
        if let Some(promo) = m.promotion() {
            s += 5_000 + VALUE[promo.index()];
        }
        scores[i] = s;
    }
}

/// Swap the best-scored move at or after `index` into `index` and return it.
#[inline]
fn pick_next(moves: &mut MoveList, scores: &mut [i32], index: usize) -> Move {
    let slice = moves.as_mut_slice();
    let mut best = index;
    for i in index + 1..slice.len() {
        if scores[i] > scores[best] {
            best = i;
        }
    }
    slice.swap(index, best);
    scores.swap(index, best);
    slice[index]
}
