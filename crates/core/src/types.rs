//! Colors, piece kinds and squares.

use std::fmt;
use std::ops::Not;
use std::str::FromStr;

/// The side to move, or the owner of a piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Color {
    /// White moves first.
    White = 0,
    /// Black.
    Black = 1,
}

impl Color {
    /// Both colors, white first.
    pub const ALL: [Color; 2] = [Color::White, Color::Black];

    /// `0` for white, `1` for black: the index into per-color arrays.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The other color.
    #[inline]
    pub const fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

impl Not for Color {
    type Output = Color;
    #[inline]
    fn not(self) -> Color {
        self.flip()
    }
}

/// A kind of piece, without its color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum PieceKind {
    /// Pawn.
    Pawn = 0,
    /// Knight.
    Knight = 1,
    /// Bishop.
    Bishop = 2,
    /// Rook.
    Rook = 3,
    /// Queen.
    Queen = 4,
    /// King.
    King = 5,
}

impl PieceKind {
    /// Every kind, pawn first.
    pub const ALL: [PieceKind; 6] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];

    /// The index into per-kind arrays.
    #[inline]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// From an index in `0..6`.
    #[inline]
    pub const fn from_index(i: usize) -> PieceKind {
        debug_assert!(i < 6);
        PieceKind::ALL[i]
    }

    /// From a FEN letter of either case.
    pub const fn from_char(c: char) -> Option<PieceKind> {
        Some(match c.to_ascii_lowercase() {
            'p' => PieceKind::Pawn,
            'n' => PieceKind::Knight,
            'b' => PieceKind::Bishop,
            'r' => PieceKind::Rook,
            'q' => PieceKind::Queen,
            'k' => PieceKind::King,
            _ => return None,
        })
    }

    /// The FEN letter: upper case for white, lower case for black.
    pub const fn to_char(self, color: Color) -> char {
        let c = match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        };
        match color {
            Color::White => c.to_ascii_uppercase(),
            Color::Black => c,
        }
    }
}

/// A square, `0..64`, a1 = 0, b1 = 1, ..., h8 = 63 (file first, then rank).
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Square(u8);

macro_rules! squares {
    ($($name:ident = $index:expr),* $(,)?) => {
        impl Square {
            $(
                #[doc = concat!("The square ", stringify!($name), ".")]
                pub const $name: Square = Square($index);
            )*
        }
    };
}

squares! {
    A1 = 0, B1 = 1, C1 = 2, D1 = 3, E1 = 4, F1 = 5, G1 = 6, H1 = 7,
    A2 = 8, B2 = 9, C2 = 10, D2 = 11, E2 = 12, F2 = 13, G2 = 14, H2 = 15,
    A3 = 16, B3 = 17, C3 = 18, D3 = 19, E3 = 20, F3 = 21, G3 = 22, H3 = 23,
    A4 = 24, B4 = 25, C4 = 26, D4 = 27, E4 = 28, F4 = 29, G4 = 30, H4 = 31,
    A5 = 32, B5 = 33, C5 = 34, D5 = 35, E5 = 36, F5 = 37, G5 = 38, H5 = 39,
    A6 = 40, B6 = 41, C6 = 42, D6 = 43, E6 = 44, F6 = 45, G6 = 46, H6 = 47,
    A7 = 48, B7 = 49, C7 = 50, D7 = 51, E7 = 52, F7 = 53, G7 = 54, H7 = 55,
    A8 = 56, B8 = 57, C8 = 58, D8 = 59, E8 = 60, F8 = 61, G8 = 62, H8 = 63,
}

impl Square {
    /// Every square, a1 first.
    pub const ALL: [Square; 64] = {
        let mut all = [Square(0); 64];
        let mut i = 0;
        while i < 64 {
            all[i] = Square(i as u8);
            i += 1;
        }
        all
    };

    /// From a file (`0` = a) and a rank (`0` = 1).
    #[inline]
    pub const fn new(file: u8, rank: u8) -> Square {
        debug_assert!(file < 8 && rank < 8);
        Square(rank * 8 + file)
    }

    /// From an index in `0..64`.
    #[inline]
    pub const fn from_index(index: u8) -> Square {
        debug_assert!(index < 64);
        Square(index)
    }

    /// The index in `0..64`.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// The file, `0` = a through `7` = h.
    #[inline]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }

    /// The rank, `0` = first through `7` = eighth.
    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }

    /// The square `df` files and `dr` ranks away, if it is on the board.
    #[inline]
    pub const fn offset(self, df: i8, dr: i8) -> Option<Square> {
        let file = self.file() as i8 + df;
        let rank = self.rank() as i8 + dr;
        if file < 0 || file > 7 || rank < 0 || rank > 7 {
            None
        } else {
            Some(Square::new(file as u8, rank as u8))
        }
    }

    /// The same file on the mirrored rank (a1 <-> a8).
    #[inline]
    pub const fn flip_rank(self) -> Square {
        Square(self.0 ^ 56)
    }
}

/// A string that is not a square name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseSquareError(pub String);

impl fmt::Display for ParseSquareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a square", self.0)
    }
}

impl std::error::Error for ParseSquareError {}

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Square, ParseSquareError> {
        let bytes = s.as_bytes();
        match bytes {
            [f @ b'a'..=b'h', r @ b'1'..=b'8'] => Ok(Square::new(f - b'a', r - b'1')),
            _ => Err(ParseSquareError(s.to_string())),
        }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", (b'a' + self.file()) as char, self.rank() + 1)
    }
}

impl fmt::Debug for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
