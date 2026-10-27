//! A set of squares as 64 bits, bit `i` = square `i` (a1 = bit 0).

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Shl, Shr};

use crate::types::{Color, Square};

/// A set of squares.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Bitboard(pub u64);

impl Bitboard {
    /// No squares.
    pub const EMPTY: Bitboard = Bitboard(0);
    /// Every square.
    pub const ALL: Bitboard = Bitboard(u64::MAX);
    /// The a-file.
    pub const FILE_A: Bitboard = Bitboard(0x0101_0101_0101_0101);
    /// The b-file.
    pub const FILE_B: Bitboard = Bitboard(Self::FILE_A.0 << 1);
    /// The c-file.
    pub const FILE_C: Bitboard = Bitboard(Self::FILE_A.0 << 2);
    /// The d-file.
    pub const FILE_D: Bitboard = Bitboard(Self::FILE_A.0 << 3);
    /// The e-file.
    pub const FILE_E: Bitboard = Bitboard(Self::FILE_A.0 << 4);
    /// The f-file.
    pub const FILE_F: Bitboard = Bitboard(Self::FILE_A.0 << 5);
    /// The g-file.
    pub const FILE_G: Bitboard = Bitboard(Self::FILE_A.0 << 6);
    /// The h-file.
    pub const FILE_H: Bitboard = Bitboard(Self::FILE_A.0 << 7);
    /// The first rank.
    pub const RANK_1: Bitboard = Bitboard(0xFF);
    /// The second rank.
    pub const RANK_2: Bitboard = Bitboard(0xFF << 8);
    /// The third rank.
    pub const RANK_3: Bitboard = Bitboard(0xFF << 16);
    /// The fourth rank.
    pub const RANK_4: Bitboard = Bitboard(0xFF << 24);
    /// The fifth rank.
    pub const RANK_5: Bitboard = Bitboard(0xFF << 32);
    /// The sixth rank.
    pub const RANK_6: Bitboard = Bitboard(0xFF << 40);
    /// The seventh rank.
    pub const RANK_7: Bitboard = Bitboard(0xFF << 48);
    /// The eighth rank.
    pub const RANK_8: Bitboard = Bitboard(0xFF << 56);

    /// The file `0` = a through `7` = h.
    #[inline]
    pub const fn file(file: u8) -> Bitboard {
        Bitboard(Self::FILE_A.0 << file)
    }

    /// The rank `0` = first through `7` = eighth.
    #[inline]
    pub const fn rank(rank: u8) -> Bitboard {
        Bitboard(0xFF << (rank * 8))
    }

    /// Just `sq`.
    #[inline]
    pub const fn from_square(sq: Square) -> Bitboard {
        Bitboard(1 << sq.index())
    }

    /// Whether `sq` is in the set.
    #[inline]
    pub const fn contains(self, sq: Square) -> bool {
        self.0 & (1 << sq.index()) != 0
    }

    /// The set plus `sq`.
    #[inline]
    pub const fn with(self, sq: Square) -> Bitboard {
        Bitboard(self.0 | (1 << sq.index()))
    }

    /// The set minus `sq`.
    #[inline]
    pub const fn without(self, sq: Square) -> Bitboard {
        Bitboard(self.0 & !(1 << sq.index()))
    }

    /// How many squares.
    #[inline]
    pub const fn count(self) -> u32 {
        self.0.count_ones()
    }

    /// Whether there are none.
    #[inline]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Whether there is at least one.
    #[inline]
    pub const fn is_nonempty(self) -> bool {
        self.0 != 0
    }

    /// Whether there is more than one.
    #[inline]
    pub const fn more_than_one(self) -> bool {
        self.0 & self.0.wrapping_sub(1) != 0
    }

    /// The lowest square (a1 is lowest), if any.
    #[inline]
    pub const fn lsb(self) -> Option<Square> {
        if self.0 == 0 {
            None
        } else {
            Some(Square::from_index(self.0.trailing_zeros() as u8))
        }
    }

    /// Remove and return the lowest square.
    #[inline]
    pub fn pop_lsb(&mut self) -> Option<Square> {
        let sq = self.lsb()?;
        self.0 &= self.0 - 1;
        Some(sq)
    }

    /// One rank up (toward the eighth); the eighth rank falls off.
    #[inline]
    pub const fn north(self) -> Bitboard {
        Bitboard(self.0 << 8)
    }

    /// One rank down.
    #[inline]
    pub const fn south(self) -> Bitboard {
        Bitboard(self.0 >> 8)
    }

    /// One file toward h; the h-file falls off.
    #[inline]
    pub const fn east(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_H.0) << 1)
    }

    /// One file toward a.
    #[inline]
    pub const fn west(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_A.0) >> 1)
    }

    /// Up and toward h.
    #[inline]
    pub const fn north_east(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_H.0) << 9)
    }

    /// Up and toward a.
    #[inline]
    pub const fn north_west(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_A.0) << 7)
    }

    /// Down and toward h.
    #[inline]
    pub const fn south_east(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_H.0) >> 7)
    }

    /// Down and toward a.
    #[inline]
    pub const fn south_west(self) -> Bitboard {
        Bitboard((self.0 & !Self::FILE_A.0) >> 9)
    }

    /// One rank forward for `color`: north for white, south for black.
    #[inline]
    pub const fn shift(self, color: Color) -> Bitboard {
        match color {
            Color::White => self.north(),
            Color::Black => self.south(),
        }
    }
}

/// Iterates the squares from a1 upward.
pub struct Squares(Bitboard);

impl Iterator for Squares {
    type Item = Square;
    #[inline]
    fn next(&mut self) -> Option<Square> {
        self.0.pop_lsb()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.0.count() as usize;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Squares {}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Squares;
    #[inline]
    fn into_iter(self) -> Squares {
        Squares(self)
    }
}

macro_rules! ops {
    ($($trait:ident :: $method:ident => $assign:ident :: $assign_method:ident ($op:tt)),* $(,)?) => {$(
        impl $trait for Bitboard {
            type Output = Bitboard;
            #[inline]
            fn $method(self, rhs: Bitboard) -> Bitboard { Bitboard(self.0 $op rhs.0) }
        }
        impl $assign for Bitboard {
            #[inline]
            fn $assign_method(&mut self, rhs: Bitboard) { self.0 = self.0 $op rhs.0; }
        }
    )*};
}

ops! {
    BitAnd::bitand => BitAndAssign::bitand_assign (&),
    BitOr::bitor => BitOrAssign::bitor_assign (|),
    BitXor::bitxor => BitXorAssign::bitxor_assign (^),
}

impl Not for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn not(self) -> Bitboard {
        Bitboard(!self.0)
    }
}

impl Shl<u32> for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn shl(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<u32> for Bitboard {
    type Output = Bitboard;
    #[inline]
    fn shr(self, rhs: u32) -> Bitboard {
        Bitboard(self.0 >> rhs)
    }
}

impl fmt::Display for Bitboard {
    /// Eight rows from the eighth rank down, `X` for a set square.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let c = if self.contains(Square::new(file, rank)) {
                    'X'
                } else {
                    '.'
                };
                f.write_str(if c == 'X' { "X" } else { "." })?;
            }
            if rank > 0 {
                f.write_str("\n")?;
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bitboard({:#018x})", self.0)
    }
}
