//! Iterative deepening, fail-soft negamax alpha-beta, and a captures-only
//! quiescence search.
//!
//! Phase 2 gave the baseline; Phase 3 adds the transposition table (probed
//! at every main-search node, a bound stored from the score against the
//! original window, never after an abort) and the staged move picker (TT
//! move, good captures by static exchange, killers, history quiets, bad
//! captures). No pruning yet.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::{MoveList, generate_captures};
use rusty_chess_core::types::PieceKind;

use crate::board::Board;
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

    /// Clear the transposition table (`ucinewgame`, and between bench positions).
    pub fn clear_hash(&mut self) {
        self.tt.clear();
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
            let score = self.negamax(board, depth, -INFINITY, INFINITY, 0);
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
        beta: i32,
        ply: usize,
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

        let mut picker = MovePicker::new(tt_move).with_killers(self.killers.get(ply));
        let mut best = -INFINITY;
        let mut best_move = Move::NULL;
        let mut index = 0usize;
        while let Some(m) = picker.next(board.position()) {
            let undo = board.make_move(m);
            let score = -self.negamax(board, depth - 1, -beta, -alpha, ply + 1);
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
                        }
                        break;
                    }
                }
            }
            index += 1;
        }
        if index == 0 && best == -INFINITY {
            // No legal moves at all: mate or stalemate.
            return if board.position().in_check() {
                -(MATE - ply as i32)
            } else {
                0
            };
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
