//! Pawn-structure correction history (`docs/correction-history.md`).
//!
//! A table, per side to move and pawn structure, of how far the network's
//! static evaluation has tended to sit from what the search then found in
//! positions with that structure. The search adds the entry to the raw
//! evaluation before pruning reads it, and updates the entry where it stores
//! a node's result. The design, its constants and the measurement behind
//! them were fixed before the code, in the design note.

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};

use crate::search::MATE_BOUND;
use crate::tt::Bound;

/// Entries a side: the top `SIZE_BITS` bits of the structure key.
pub const SIZE: usize = 1 << SIZE_BITS;
const SIZE_BITS: u32 = 14;

/// Stored values are in `1 / GRAIN` centipawns.
pub const GRAIN: i32 = 256;

/// The most a correction may move an evaluation, in centipawns.
pub const MAX_CORRECTION: i32 = 512;

/// The most an update weighs, of `WEIGHT_SCALE`.
pub const MAX_WEIGHT: i32 = 16;
const WEIGHT_SCALE: i32 = 64;

/// Corrections for `[side to move][structure]`, in `1 / GRAIN` centipawns,
/// each within `±MAX_CORRECTION × GRAIN`.
#[derive(Clone)]
pub struct CorrectionHistory {
    table: Box<[[i32; SIZE]; 2]>,
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
            table: Box::new([[0; SIZE]; 2]),
        }
    }

    /// Forget everything (a new game).
    pub fn clear(&mut self) {
        *self.table = [[0; SIZE]; 2];
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

    #[inline]
    fn slot(pos: &Position) -> (usize, usize) {
        (
            pos.side_to_move().index(),
            (Self::key(pos) >> (64 - SIZE_BITS)) as usize,
        )
    }

    /// The correction for `pos`'s side to move, in centipawns.
    #[inline]
    pub fn correction(&self, pos: &Position) -> i32 {
        let (side, i) = Self::slot(pos);
        self.table[side][i] / GRAIN
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

    /// Move the entry for `pos` towards `score − raw`, by a weight that grows
    /// with `depth`. The target is clamped to the cap, so the entry can never
    /// pass it; integer division also stops it a little short. Returns
    /// whether the error was beyond the cap.
    #[inline]
    pub fn update(&mut self, pos: &Position, raw: i32, score: i32, depth: u32) -> bool {
        let error = score - raw;
        let target = error.clamp(-MAX_CORRECTION, MAX_CORRECTION) * GRAIN;
        let weight = (depth as i32 + 1).min(MAX_WEIGHT);
        let (side, i) = Self::slot(pos);
        let entry = &mut self.table[side][i];
        *entry += (target - *entry) * weight / WEIGHT_SCALE;
        error.abs() > MAX_CORRECTION
    }
}
