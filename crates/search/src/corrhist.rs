//! Correction history (`docs/correction-history.md`): two tables of how far
//! the network's static evaluation has tended to sit from what the search
//! then found, one keyed by the pawn structure and one by where the major
//! pieces (rooks, queens, kings) stand, each per side to move. The search
//! adds their sum to the raw evaluation before pruning reads it, and updates
//! both where it stores a node's result. The design, its constants and the
//! measurements behind them were fixed before the code, in the design note.
//!
//! The pawn table learns the evaluation's error; the major-piece table learns
//! the error the pawn table leaves, so the two don't learn the same error
//! twice. The applied sum is clamped to the same cap as one table, so adding
//! the second changed where corrections come from, not how large they can be.

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};

use crate::search::MATE_BOUND;
use crate::tt::Bound;

/// Entries a side in each table: the top `SIZE_BITS` bits of the key.
pub const SIZE: usize = 1 << SIZE_BITS;
const SIZE_BITS: u32 = 14;

/// Stored values are in `1 / GRAIN` centipawns.
pub const GRAIN: i32 = 256;

/// The most a correction may move an evaluation, in centipawns: each table's
/// entries are held within it, and so is the applied sum.
pub const MAX_CORRECTION: i32 = 128;

/// The most an update weighs, of `WEIGHT_SCALE`.
pub const MAX_WEIGHT: i32 = 16;
const WEIGHT_SCALE: i32 = 256;

/// One table of corrections for `[side to move][key]`, in `1 / GRAIN`
/// centipawns, each within `±MAX_CORRECTION × GRAIN`.
#[derive(Clone)]
struct Table(Box<[[i32; SIZE]; 2]>);

impl Table {
    fn new() -> Table {
        Table(Box::new([[0; SIZE]; 2]))
    }

    fn clear(&mut self) {
        *self.0 = [[0; SIZE]; 2];
    }

    #[inline]
    fn slot(pos: &Position, key: u64) -> (usize, usize) {
        (
            pos.side_to_move().index(),
            (key >> (64 - SIZE_BITS)) as usize,
        )
    }

    /// The entry for `pos` under `key`, in centipawns.
    #[inline]
    fn correction(&self, pos: &Position, key: u64) -> i32 {
        let (side, i) = Self::slot(pos, key);
        self.0[side][i] / GRAIN
    }

    /// Move the entry towards `error`, by a weight that grows with `depth`.
    /// The target is clamped to the cap, so the entry can never pass it;
    /// integer division also stops it a little short. Returns whether the
    /// error was beyond the cap.
    #[inline]
    fn update(&mut self, pos: &Position, key: u64, error: i32, depth: u32) -> bool {
        let target = error.clamp(-MAX_CORRECTION, MAX_CORRECTION) * GRAIN;
        let weight = (depth as i32 + 1).min(MAX_WEIGHT);
        let (side, i) = Self::slot(pos, key);
        let entry = &mut self.0[side][i];
        *entry += (target - *entry) * weight / WEIGHT_SCALE;
        error.abs() > MAX_CORRECTION
    }
}

/// Mix bitboards into a key. Each gets its own odd multiplier and the mix is
/// rotated between them, so the same board in another slot gives another key.
#[inline]
fn mix(bitboards: &[u64]) -> u64 {
    let mut key = 0x9E37_79B9_7F4A_7C15u64;
    for (i, b) in bitboards.iter().enumerate() {
        key ^= b.wrapping_mul(
            0xC2B2_AE3D_27D4_EB4F ^ (i as u64 + 1).wrapping_mul(0x1656_67B1_9E37_79F9),
        );
        key = key.rotate_left(27);
    }
    key
}

/// The pawn table and the major-piece table, read and updated together.
#[derive(Clone)]
pub struct CorrectionHistory {
    pawns: Table,
    majors: Table,
}

impl Default for CorrectionHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CorrectionHistory {
    /// All zero: no correction anywhere.
    pub fn new() -> CorrectionHistory {
        CorrectionHistory {
            pawns: Table::new(),
            majors: Table::new(),
        }
    }

    /// Forget everything (a new game).
    pub fn clear(&mut self) {
        self.pawns.clear();
        self.majors.clear();
    }

    /// The pawn structure's key: both sides' pawn bitboards mixed. There is
    /// no pawn Zobrist key to reuse, and two multiplies cost less than one.
    #[inline]
    pub fn key(pos: &Position) -> u64 {
        let white = pos.pieces(Color::White, PieceKind::Pawn).0;
        let black = pos.pieces(Color::Black, PieceKind::Pawn).0;
        white.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ black.rotate_left(32).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
    }

    /// The major pieces' key: each side's rooks, queens and king mixed.
    #[inline]
    pub fn major_key(pos: &Position) -> u64 {
        let bb = |c: Color, k: PieceKind| pos.pieces(c, k).0;
        mix(&[
            bb(Color::White, PieceKind::Rook),
            bb(Color::White, PieceKind::Queen),
            bb(Color::White, PieceKind::King),
            bb(Color::Black, PieceKind::Rook),
            bb(Color::Black, PieceKind::Queen),
            bb(Color::Black, PieceKind::King),
        ])
    }

    /// The pawn table's correction for `pos`'s side to move, in centipawns.
    #[inline]
    pub fn pawn_correction(&self, pos: &Position) -> i32 {
        self.pawns.correction(pos, Self::key(pos))
    }

    /// The major-piece table's correction for `pos`'s side to move, in
    /// centipawns.
    #[inline]
    pub fn major_correction(&self, pos: &Position) -> i32 {
        self.majors.correction(pos, Self::major_key(pos))
    }

    /// The correction applied for `pos`'s side to move, in centipawns: both
    /// tables' entries summed and held within the cap.
    #[inline]
    pub fn correction(&self, pos: &Position) -> i32 {
        (self.pawn_correction(pos) + self.major_correction(pos))
            .clamp(-MAX_CORRECTION, MAX_CORRECTION)
    }

    /// Whether the two entries' sum for `pos` lies beyond the cap, so the
    /// applied correction is the cap (for the counters).
    #[inline]
    pub fn sum_clamped(&self, pos: &Position) -> bool {
        (self.pawn_correction(pos) + self.major_correction(pos)).abs() > MAX_CORRECTION
    }

    /// `raw`, the network's evaluation for the side to move, corrected, and
    /// kept below the mate range whatever the correction.
    #[inline]
    pub fn corrected(&self, pos: &Position, raw: i32) -> i32 {
        (raw + self.correction(pos)).clamp(-(MATE_BOUND - 1), MATE_BOUND - 1)
    }

    /// Whether a stored result is evidence about the evaluation's error: an
    /// exact score always is; a fail-high only says the position is at least
    /// `score`, so it counts only above the evaluation the search used, and
    /// a fail-low only below it.
    #[inline]
    pub fn is_evidence(bound: Bound, score: i32, corrected: i32) -> bool {
        match bound {
            Bound::Exact => true,
            Bound::Lower => score > corrected,
            Bound::Upper => score < corrected,
        }
    }

    /// Learn from a stored result: the pawn table moves towards
    /// `score − raw`, the evaluation's error, and the major-piece table
    /// towards what the pawn table's entry (as it stood before this update)
    /// left of it. Returns whether each table's error was beyond the cap,
    /// `(pawns, majors)`.
    #[inline]
    pub fn update(&mut self, pos: &Position, raw: i32, score: i32, depth: u32) -> (bool, bool) {
        let key = Self::key(pos);
        let pawn = self.pawns.correction(pos, key);
        let capped_pawns = self.pawns.update(pos, key, score - raw, depth);
        let capped_majors =
            self.majors
                .update(pos, Self::major_key(pos), score - (raw + pawn), depth);
        (capped_pawns, capped_majors)
    }
}
