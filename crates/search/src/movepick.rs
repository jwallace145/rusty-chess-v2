//! The staged move picker.
//!
//! Moves come out in the order most likely to cut off: the transposition
//! table move, captures with non-negative static exchange value ordered by
//! victim then attacker, killers, the remaining quiets ordered by history,
//! and finally the losing captures. Each move is scored once.

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::{MAX_MOVES, MoveList, generate_captures, generate_quiets};
use rusty_chess_core::position::Position;

use crate::see::{SEE_VALUE, see};
pub use crate::stats::Stage;

/// Yields the legal moves of a position stage by stage.
pub struct MovePicker {
    tt_move: Move,
    stage: Stage,
    state: State,
    captures: MoveList,
    capture_scores: [i32; MAX_MOVES],
    capture_index: usize,
    bad_captures: MoveList,
    bad_index: usize,
    quiets: MoveList,
    quiet_scores: [i32; MAX_MOVES],
    quiet_index: usize,
    killers: [Move; 2],
    killer_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    TtMove,
    GenerateCaptures,
    GoodCaptures,
    Killers,
    GenerateQuiets,
    Quiets,
    BadCaptures,
    Done,
}

impl MovePicker {
    /// A picker that yields `tt_move` first if it is legal, then the rest.
    pub fn new(tt_move: Move) -> MovePicker {
        MovePicker {
            tt_move,
            stage: Stage::TtMove,
            state: State::TtMove,
            captures: MoveList::new(),
            capture_scores: [0; MAX_MOVES],
            capture_index: 0,
            bad_captures: MoveList::new(),
            bad_index: 0,
            quiets: MoveList::new(),
            quiet_scores: [0; MAX_MOVES],
            quiet_index: 0,
            killers: [Move::NULL; 2],
            killer_index: 0,
        }
    }

    /// Set the killer moves for this ply (searched after the good captures).
    pub fn with_killers(mut self, killers: [Move; 2]) -> MovePicker {
        self.killers = killers;
        self
    }

    /// The stage the last yielded move came from.
    #[inline]
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// The next move, or `None` when every legal move has been yielded,
    /// ordering quiets without history.
    pub fn next(&mut self, pos: &Position) -> Option<Move> {
        self.next_with(pos, &|_| 0)
    }

    /// The next move, ordering the remaining quiets by `score` (history,
    /// continuation history, whatever the caller combines), evaluated once
    /// per quiet when the quiet stage begins.
    pub fn next_with(&mut self, pos: &Position, score: &dyn Fn(Move) -> i32) -> Option<Move> {
        loop {
            match self.state {
                State::TtMove => {
                    self.state = State::GenerateCaptures;
                    // Legality is checked against the generated lists below;
                    // the TT move is yielded first only once it is found.
                    self.generate(pos);
                    if self.tt_move != Move::NULL && self.contains(self.tt_move) {
                        self.stage = Stage::TtMove;
                        return Some(self.tt_move);
                    }
                }
                State::GenerateCaptures => {
                    self.score_captures(pos);
                    self.state = State::GoodCaptures;
                }
                State::GoodCaptures => {
                    while self.capture_index < self.captures.len() {
                        let m = self.pick_best_capture();
                        self.capture_index += 1;
                        if m == self.tt_move {
                            continue;
                        }
                        if see(pos, m) < 0 {
                            self.bad_captures.push(m);
                            continue;
                        }
                        self.stage = Stage::GoodCaptures;
                        return Some(m);
                    }
                    self.state = State::Killers;
                }
                State::Killers => {
                    while self.killer_index < 2 {
                        let m = self.killers[self.killer_index];
                        self.killer_index += 1;
                        if m != Move::NULL
                            && m != self.tt_move
                            && self.quiets.iter().any(|&q| q == m)
                        {
                            self.stage = Stage::Killers;
                            return Some(m);
                        }
                    }
                    self.state = State::GenerateQuiets;
                }
                State::GenerateQuiets => {
                    for (i, &m) in self.quiets.iter().enumerate() {
                        self.quiet_scores[i] = score(m);
                    }
                    self.state = State::Quiets;
                }
                State::Quiets => {
                    while self.quiet_index < self.quiets.len() {
                        let m = self.pick_best_quiet();
                        self.quiet_index += 1;
                        if m == self.tt_move || self.killers.contains(&m) {
                            continue;
                        }
                        self.stage = Stage::HistoryQuiets;
                        return Some(m);
                    }
                    self.state = State::BadCaptures;
                }
                State::BadCaptures => {
                    if self.bad_index < self.bad_captures.len() {
                        let m = self.bad_captures.as_slice()[self.bad_index];
                        self.bad_index += 1;
                        self.stage = Stage::BadCaptures;
                        return Some(m);
                    }
                    self.state = State::Done;
                }
                State::Done => return None,
            }
        }
    }

    fn generate(&mut self, pos: &Position) {
        generate_captures(pos, &mut self.captures);
        generate_quiets(pos, &mut self.quiets);
    }

    #[inline]
    fn contains(&self, m: Move) -> bool {
        self.captures.iter().any(|&c| c == m) || self.quiets.iter().any(|&q| q == m)
    }

    /// Most valuable victim, least valuable attacker; promotions count the
    /// promoted piece.
    fn score_captures(&mut self, pos: &Position) {
        for (i, &m) in self.captures.iter().enumerate() {
            let victim = if m.is_en_passant() {
                SEE_VALUE[0]
            } else {
                pos.piece_at(m.to())
                    .map_or(0, |p| SEE_VALUE[p.kind.index()])
            };
            let attacker = pos
                .piece_at(m.from())
                .map_or(0, |p| SEE_VALUE[p.kind.index()]);
            let promo = m.promotion().map_or(0, |k| SEE_VALUE[k.index()]);
            self.capture_scores[i] = 10 * victim - attacker + promo;
        }
    }

    /// Swap the best-scored remaining quiet into the current index.
    #[inline]
    fn pick_best_quiet(&mut self) -> Move {
        let index = self.quiet_index;
        let slice = self.quiets.as_mut_slice();
        let mut best = index;
        for i in index + 1..slice.len() {
            if self.quiet_scores[i] > self.quiet_scores[best] {
                best = i;
            }
        }
        slice.swap(index, best);
        self.quiet_scores.swap(index, best);
        slice[index]
    }

    /// Swap the best-scored remaining capture into the current index.
    #[inline]
    fn pick_best_capture(&mut self) -> Move {
        let index = self.capture_index;
        let slice = self.captures.as_mut_slice();
        let mut best = index;
        for i in index + 1..slice.len() {
            if self.capture_scores[i] > self.capture_scores[best] {
                best = i;
            }
        }
        slice.swap(index, best);
        self.capture_scores.swap(index, best);
        slice[index]
    }
}
