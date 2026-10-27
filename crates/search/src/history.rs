//! Butterfly history with gravity.
//!
//! A score per side, from-square and to-square for quiet moves. A quiet
//! move that causes a beta cutoff gets a bonus that grows with depth; the
//! quiet moves searched before it at that node get the same amount as a
//! malus. Gravity, `h += bonus - h * |bonus| / MAX`, keeps every score in
//! `[-MAX, MAX]` and makes recent evidence count more than old evidence.

use rusty_chess_core::r#move::Move;
use rusty_chess_core::types::Color;

/// The bound every score stays within.
pub const MAX_HISTORY: i32 = 16_384;

/// Scores for `[side][from][to]`.
#[derive(Clone)]
pub struct History {
    table: Box<[[[i32; 64]; 64]; 2]>,
}

/// An all-zero table for callers that order quiets without history.
pub static EMPTY: std::sync::LazyLock<History> = std::sync::LazyLock::new(History::new);

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    /// All zero.
    pub fn new() -> History {
        History {
            table: Box::new([[[0; 64]; 64]; 2]),
        }
    }

    /// Forget everything (a new game).
    pub fn clear(&mut self) {
        *self.table = [[[0; 64]; 64]; 2];
    }

    /// Halve every score (a new search): old evidence fades but is kept.
    pub fn age(&mut self) {
        for side in self.table.iter_mut() {
            for from in side.iter_mut() {
                for v in from.iter_mut() {
                    *v /= 2;
                }
            }
        }
    }

    /// The score of quiet move `m` for `side`.
    #[inline]
    pub fn get(&self, side: Color, m: Move) -> i32 {
        self.table[side.index()][m.from().index()][m.to().index()]
    }

    /// The bonus for a cutoff at `depth`: depth squared, capped.
    #[inline]
    pub fn bonus(depth: u32) -> i32 {
        let d = depth.min(32) as i32;
        (d * d).min(1200)
    }

    /// Apply `delta` (positive bonus, negative malus) with gravity.
    #[inline]
    pub fn update(&mut self, side: Color, m: Move, delta: i32) {
        let delta = delta.clamp(-MAX_HISTORY, MAX_HISTORY);
        let entry = &mut self.table[side.index()][m.from().index()][m.to().index()];
        *entry += delta - *entry * delta.abs() / MAX_HISTORY;
    }
}
