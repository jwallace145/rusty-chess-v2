//! A move packed into 16 bits: 6 bits from, 6 bits to, 4 bits of flag.

use std::fmt;

use crate::types::{PieceKind, Square};

/// What kind of move it is. Promotions encode the piece; bit 2 (`4`) means a
/// capture, bit 3 (`8`) a promotion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MoveFlag {
    /// A non-capturing, non-special move.
    Quiet = 0,
    /// A pawn's two-square advance.
    DoublePush = 1,
    /// Castling toward the h-file.
    KingCastle = 2,
    /// Castling toward the a-file.
    QueenCastle = 3,
    /// A capture.
    Capture = 4,
    /// An en passant capture.
    EnPassant = 5,
    /// Promotion to a knight.
    PromoKnight = 8,
    /// Promotion to a bishop.
    PromoBishop = 9,
    /// Promotion to a rook.
    PromoRook = 10,
    /// Promotion to a queen.
    PromoQueen = 11,
    /// Capture with promotion to a knight.
    PromoCaptureKnight = 12,
    /// Capture with promotion to a bishop.
    PromoCaptureBishop = 13,
    /// Capture with promotion to a rook.
    PromoCaptureRook = 14,
    /// Capture with promotion to a queen.
    PromoCaptureQueen = 15,
}

impl MoveFlag {
    const FROM_BITS: [MoveFlag; 16] = [
        MoveFlag::Quiet,
        MoveFlag::DoublePush,
        MoveFlag::KingCastle,
        MoveFlag::QueenCastle,
        MoveFlag::Capture,
        MoveFlag::EnPassant,
        MoveFlag::Quiet, // 6 unused
        MoveFlag::Quiet, // 7 unused
        MoveFlag::PromoKnight,
        MoveFlag::PromoBishop,
        MoveFlag::PromoRook,
        MoveFlag::PromoQueen,
        MoveFlag::PromoCaptureKnight,
        MoveFlag::PromoCaptureBishop,
        MoveFlag::PromoCaptureRook,
        MoveFlag::PromoCaptureQueen,
    ];

    /// The promotion flag for `kind`, with or without a capture.
    #[inline]
    pub const fn promotion(kind: PieceKind, capture: bool) -> MoveFlag {
        let bits = 8 | (if capture { 4 } else { 0 }) | (kind as u8 - PieceKind::Knight as u8);
        Self::FROM_BITS[bits as usize]
    }
}

/// A move. `Move::NULL` is the all-zero "no move".
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Move(u16);

impl Move {
    /// The null move, `a1a1` quiet; prints as `0000`.
    pub const NULL: Move = Move(0);

    /// Pack a move.
    #[inline]
    pub const fn new(from: Square, to: Square, flag: MoveFlag) -> Move {
        Move((from.index() as u16) | ((to.index() as u16) << 6) | ((flag as u16) << 12))
    }

    /// The origin square.
    #[inline]
    pub const fn from(self) -> Square {
        Square::from_index((self.0 & 63) as u8)
    }

    /// The destination square (for castling, the king's destination).
    #[inline]
    pub const fn to(self) -> Square {
        Square::from_index(((self.0 >> 6) & 63) as u8)
    }

    /// The flag.
    #[inline]
    pub const fn flag(self) -> MoveFlag {
        MoveFlag::FROM_BITS[(self.0 >> 12) as usize]
    }

    /// Whether a piece is captured, en passant included.
    #[inline]
    pub const fn is_capture(self) -> bool {
        self.0 & (4 << 12) != 0
    }

    /// Whether a pawn promotes.
    #[inline]
    pub const fn is_promotion(self) -> bool {
        self.0 & (8 << 12) != 0
    }

    /// Whether this is an en passant capture.
    #[inline]
    pub const fn is_en_passant(self) -> bool {
        (self.0 >> 12) == MoveFlag::EnPassant as u16
    }

    /// Whether this is castling, either side.
    #[inline]
    pub const fn is_castle(self) -> bool {
        let f = self.0 >> 12;
        f == MoveFlag::KingCastle as u16 || f == MoveFlag::QueenCastle as u16
    }

    /// Whether this is a pawn's two-square advance.
    #[inline]
    pub const fn is_double_push(self) -> bool {
        (self.0 >> 12) == MoveFlag::DoublePush as u16
    }

    /// The piece promoted to, if any.
    #[inline]
    pub const fn promotion(self) -> Option<PieceKind> {
        if self.is_promotion() {
            Some(PieceKind::from_index(((self.0 >> 12) & 3) as usize + 1))
        } else {
            None
        }
    }

    /// The raw 16 bits, for the transposition table.
    #[inline]
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// From raw bits.
    #[inline]
    pub const fn from_bits(bits: u16) -> Move {
        Move(bits)
    }
}

impl fmt::Display for Move {
    /// UCI notation: `e2e4`, `e7e8q`; the null move is `0000`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Move::NULL {
            return f.write_str("0000");
        }
        write!(f, "{}{}", self.from(), self.to())?;
        if let Some(kind) = self.promotion() {
            write!(f, "{}", kind.to_char(crate::types::Color::Black))?;
        }
        Ok(())
    }
}

impl fmt::Debug for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:?})", self, self.flag())
    }
}
