//! Butterfly history with gravity, and continuation history.
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

/// A move as the continuation tables see it: the piece that moved (a
/// [`Piece::index`](rusty_chess_core::position::Piece::index), 0..12) and
/// its destination. [`PieceTo::NONE`] stands for "no previous move" (the
/// root, or a null move) and never matches anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PieceTo {
    /// The moved piece's index, or 12 for none.
    pub piece: u8,
    /// The destination square's index.
    pub to: u8,
}

impl PieceTo {
    /// No previous move.
    pub const NONE: PieceTo = PieceTo { piece: 12, to: 0 };

    /// The moved piece and destination of `m` in `pos`, before it is made.
    #[inline]
    pub fn of(pos: &rusty_chess_core::position::Position, m: Move) -> PieceTo {
        match pos.piece_at(m.from()) {
            Some(p) => PieceTo {
                piece: (p.color.index() * 6 + p.kind.index()) as u8,
                to: m.to().index() as u8,
            },
            None => PieceTo::NONE,
        }
    }

    #[inline]
    fn is_none(self) -> bool {
        self.piece >= 12
    }
}

/// Continuation history: a score for a quiet move given the move played a
/// fixed number of plies earlier, indexed `[earlier piece][earlier to][piece][to]`.
/// One table per distance (one ply back is the opponent's reply, two plies
/// back our own previous move). Same bonus, malus and gravity as [`History`].
#[derive(Clone)]
pub struct Continuation {
    table: Box<[[[[i32; 64]; 12]; 64]; 12]>,
}

impl Default for Continuation {
    fn default() -> Self {
        Self::new()
    }
}

impl Continuation {
    /// All zero.
    pub fn new() -> Continuation {
        Continuation {
            table: vec![[[[0i32; 64]; 12]; 64]; 12]
                .into_boxed_slice()
                .try_into()
                .unwrap_or_else(|_| unreachable!("12 entries")),
        }
    }

    /// Forget everything (a new game).
    pub fn clear(&mut self) {
        for a in self.table.iter_mut() {
            for b in a.iter_mut() {
                for c in b.iter_mut() {
                    c.fill(0);
                }
            }
        }
    }

    /// Halve every score (a new search).
    pub fn age(&mut self) {
        for a in self.table.iter_mut() {
            for b in a.iter_mut() {
                for c in b.iter_mut() {
                    for v in c.iter_mut() {
                        *v /= 2;
                    }
                }
            }
        }
    }

    /// The score of playing `m` after `prev`; zero with no previous move.
    #[inline]
    pub fn get(&self, prev: PieceTo, m: PieceTo) -> i32 {
        if prev.is_none() || m.is_none() {
            return 0;
        }
        self.table[prev.piece as usize][prev.to as usize][m.piece as usize][m.to as usize]
    }

    /// Apply `delta` with gravity; nothing with no previous move.
    #[inline]
    pub fn update(&mut self, prev: PieceTo, m: PieceTo, delta: i32) {
        if prev.is_none() || m.is_none() {
            return;
        }
        let delta = delta.clamp(-MAX_HISTORY, MAX_HISTORY);
        let entry =
            &mut self.table[prev.piece as usize][prev.to as usize][m.piece as usize][m.to as usize];
        *entry += delta - *entry * delta.abs() / MAX_HISTORY;
    }
}
