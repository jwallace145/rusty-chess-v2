//! The position: piece placement as bitboards plus a mailbox, the side to
//! move, castling rights, the en passant square, the clocks, and the
//! incrementally maintained Zobrist key.

use std::fmt;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

use crate::attacks;
use crate::bitboard::Bitboard;
use crate::r#move::{Move, MoveFlag};
use crate::types::{Color, PieceKind, Square};
use crate::zobrist;

/// A colored piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Piece {
    /// Whose it is.
    pub color: Color,
    /// What it is.
    pub kind: PieceKind,
}

impl Piece {
    /// A piece of `color` and `kind`.
    #[inline]
    pub const fn new(color: Color, kind: PieceKind) -> Piece {
        Piece { color, kind }
    }

    /// From a FEN letter.
    pub const fn from_char(c: char) -> Option<Piece> {
        match PieceKind::from_char(c) {
            Some(kind) => Some(Piece::new(
                if c.is_ascii_uppercase() {
                    Color::White
                } else {
                    Color::Black
                },
                kind,
            )),
            None => None,
        }
    }

    /// The FEN letter.
    pub const fn to_char(self) -> char {
        self.kind.to_char(self.color)
    }
}

/// Castling rights as four bits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CastleRights(u8);

impl CastleRights {
    /// No castling.
    pub const NONE: CastleRights = CastleRights(0);
    /// White may castle toward the h-file.
    pub const WHITE_KING: CastleRights = CastleRights(1);
    /// White may castle toward the a-file.
    pub const WHITE_QUEEN: CastleRights = CastleRights(2);
    /// Black may castle toward the h-file.
    pub const BLACK_KING: CastleRights = CastleRights(4);
    /// Black may castle toward the a-file.
    pub const BLACK_QUEEN: CastleRights = CastleRights(8);
    /// Everything.
    pub const ALL: CastleRights = CastleRights(15);

    /// Whether every right in `other` is present.
    #[inline]
    pub const fn contains(self, other: CastleRights) -> bool {
        self.0 & other.0 == other.0
    }

    /// The index into the Zobrist castle table.
    #[inline]
    pub const fn index(self) -> usize {
        self.0 as usize
    }

    /// Both rights of `color`.
    #[inline]
    pub const fn of(color: Color) -> CastleRights {
        match color {
            Color::White => CastleRights(3),
            Color::Black => CastleRights(12),
        }
    }
}

impl BitOr for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn bitor(self, rhs: CastleRights) -> CastleRights {
        CastleRights(self.0 | rhs.0)
    }
}

impl BitOrAssign for CastleRights {
    #[inline]
    fn bitor_assign(&mut self, rhs: CastleRights) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn bitand(self, rhs: CastleRights) -> CastleRights {
        CastleRights(self.0 & rhs.0)
    }
}

impl Not for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn not(self) -> CastleRights {
        CastleRights(!self.0 & 15)
    }
}

impl fmt::Debug for CastleRights {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CastleRights({})", self.to_fen())
    }
}

impl CastleRights {
    fn to_fen(self) -> String {
        if self == CastleRights::NONE {
            return "-".into();
        }
        let mut s = String::new();
        for (right, c) in [
            (CastleRights::WHITE_KING, 'K'),
            (CastleRights::WHITE_QUEEN, 'Q'),
            (CastleRights::BLACK_KING, 'k'),
            (CastleRights::BLACK_QUEEN, 'q'),
        ] {
            if self.contains(right) {
                s.push(c);
            }
        }
        s
    }
}

/// The rights lost when a piece moves from or to each square: the king's
/// square loses both, a rook's square loses its side.
static CASTLE_MASK: [u8; 64] = {
    let mut t = [0u8; 64];
    t[Square::A1.index()] = 2;
    t[Square::E1.index()] = 3;
    t[Square::H1.index()] = 1;
    t[Square::A8.index()] = 8;
    t[Square::E8.index()] = 12;
    t[Square::H8.index()] = 4;
    t
};

/// What `make_move` needs back to `unmake_move`.
#[derive(Clone, Copy, Debug)]
pub struct Undo {
    captured: Option<Piece>,
    castling: CastleRights,
    en_passant: Option<Square>,
    halfmove: u16,
    key: u64,
}

impl Undo {
    /// The piece the move captured, if any (for en passant, the pawn).
    #[inline]
    pub fn captured(&self) -> Option<Piece> {
        self.captured
    }
}

/// What `make_null_move` needs back to `unmake_null_move`.
#[derive(Clone, Copy, Debug)]
pub struct NullUndo {
    en_passant: Option<Square>,
    halfmove: u16,
    key: u64,
}

/// A FEN that could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FenError(pub String);

impl fmt::Display for FenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid fen: {}", self.0)
    }
}

impl std::error::Error for FenError {}

/// The position.
#[derive(Clone, PartialEq, Eq)]
pub struct Position {
    by_kind: [Bitboard; 6],
    by_color: [Bitboard; 2],
    mailbox: [Option<Piece>; 64],
    side: Color,
    castling: CastleRights,
    en_passant: Option<Square>,
    halfmove: u16,
    fullmove: u16,
    key: u64,
}

impl Position {
    /// The standard starting position.
    pub fn startpos() -> Position {
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    /// Parse a FEN. The two clocks may be omitted.
    pub fn from_fen(fen: &str) -> Result<Position, FenError> {
        let err = |m: &str| FenError(format!("{m} in `{fen}`"));
        let fields: Vec<&str> = fen.split_whitespace().collect();
        if fields.len() < 4 || fields.len() > 6 {
            return Err(err("expected 4 to 6 fields"));
        }
        let mut pos = Position {
            by_kind: [Bitboard::EMPTY; 6],
            by_color: [Bitboard::EMPTY; 2],
            mailbox: [None; 64],
            side: Color::White,
            castling: CastleRights::NONE,
            en_passant: None,
            halfmove: 0,
            fullmove: 1,
            key: 0,
        };

        let ranks: Vec<&str> = fields[0].split('/').collect();
        if ranks.len() != 8 {
            return Err(err("expected 8 ranks"));
        }
        for (i, rank_text) in ranks.iter().enumerate() {
            let rank = 7 - i as u8;
            let mut file = 0u8;
            for c in rank_text.chars() {
                if let Some(skip) = c.to_digit(10) {
                    file += skip as u8;
                } else if let Some(piece) = Piece::from_char(c) {
                    if file > 7 {
                        return Err(err("rank has more than 8 squares"));
                    }
                    pos.put(Square::new(file, rank), piece);
                    file += 1;
                } else {
                    return Err(err("unexpected character in the piece placement"));
                }
            }
            if file != 8 {
                return Err(err("rank does not have 8 squares"));
            }
        }
        for color in Color::ALL {
            if pos.pieces(color, PieceKind::King).count() != 1 {
                return Err(err("each side needs exactly one king"));
            }
        }

        pos.side = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(err("side to move must be `w` or `b`")),
        };

        if fields[2] != "-" {
            for c in fields[2].chars() {
                let right = match c {
                    'K' => CastleRights::WHITE_KING,
                    'Q' => CastleRights::WHITE_QUEEN,
                    'k' => CastleRights::BLACK_KING,
                    'q' => CastleRights::BLACK_QUEEN,
                    _ => return Err(err("unexpected castling character")),
                };
                if pos.castling.contains(right) {
                    return Err(err("duplicate castling right"));
                }
                pos.castling |= right;
            }
            // Rights without the king and rook in place are meaningless.
            for (right, king, rook, color) in [
                (
                    CastleRights::WHITE_KING,
                    Square::E1,
                    Square::H1,
                    Color::White,
                ),
                (
                    CastleRights::WHITE_QUEEN,
                    Square::E1,
                    Square::A1,
                    Color::White,
                ),
                (
                    CastleRights::BLACK_KING,
                    Square::E8,
                    Square::H8,
                    Color::Black,
                ),
                (
                    CastleRights::BLACK_QUEEN,
                    Square::E8,
                    Square::A8,
                    Color::Black,
                ),
            ] {
                if pos.castling.contains(right)
                    && (pos.piece_at(king) != Some(Piece::new(color, PieceKind::King))
                        || pos.piece_at(rook) != Some(Piece::new(color, PieceKind::Rook)))
                {
                    pos.castling = pos.castling & !right;
                }
            }
        }

        if fields[3] != "-" {
            let sq: Square = fields[3]
                .parse()
                .map_err(|_| err("bad en passant square"))?;
            let expected_rank = if pos.side == Color::White { 5 } else { 2 };
            if sq.rank() != expected_rank {
                return Err(err("en passant square on the wrong rank"));
            }
            pos.en_passant = pos.capturable_en_passant(sq);
        }

        if let Some(h) = fields.get(4) {
            pos.halfmove = h.parse().map_err(|_| err("bad halfmove clock"))?;
        }
        if let Some(f) = fields.get(5) {
            pos.fullmove = f.parse().map_err(|_| err("bad fullmove number"))?;
            if pos.fullmove == 0 {
                return Err(err("fullmove number starts at 1"));
            }
        }

        pos.key = pos.recompute_key();
        Ok(pos)
    }

    /// The FEN.
    pub fn to_fen(&self) -> String {
        let mut s = String::with_capacity(90);
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                match self.mailbox[Square::new(file, rank).index()] {
                    Some(piece) => {
                        if empty > 0 {
                            s.push(char::from(b'0' + empty));
                            empty = 0;
                        }
                        s.push(piece.to_char());
                    }
                    None => empty += 1,
                }
            }
            if empty > 0 {
                s.push(char::from(b'0' + empty));
            }
            if rank > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.side == Color::White { 'w' } else { 'b' });
        s.push(' ');
        s.push_str(&self.castling.to_fen());
        s.push(' ');
        match self.en_passant {
            Some(sq) => s.push_str(&sq.to_string()),
            None => s.push('-'),
        }
        s.push_str(&format!(" {} {}", self.halfmove, self.fullmove));
        s
    }

    /// The piece on `sq`.
    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.mailbox[sq.index()]
    }

    /// Whose turn it is.
    #[inline]
    pub fn side_to_move(&self) -> Color {
        self.side
    }

    /// The castling rights.
    #[inline]
    pub fn castling(&self) -> CastleRights {
        self.castling
    }

    /// The en passant target square, if a capture there is possible.
    #[inline]
    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    /// Plies since the last capture or pawn move.
    #[inline]
    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove
    }

    /// The move number, starting at 1 and incremented after black moves.
    #[inline]
    pub fn fullmove_number(&self) -> u16 {
        self.fullmove
    }

    /// The pieces of `color` and `kind`.
    #[inline]
    pub fn pieces(&self, color: Color, kind: PieceKind) -> Bitboard {
        self.by_kind[kind.index()] & self.by_color[color.index()]
    }

    /// Every piece of `color`.
    #[inline]
    pub fn by_color(&self, color: Color) -> Bitboard {
        self.by_color[color.index()]
    }

    /// Every piece of `kind`.
    #[inline]
    pub fn by_kind(&self, kind: PieceKind) -> Bitboard {
        self.by_kind[kind.index()]
    }

    /// Every piece.
    #[inline]
    pub fn occupied(&self) -> Bitboard {
        self.by_color[0] | self.by_color[1]
    }

    /// Where `color`'s king is.
    #[inline]
    pub fn king_square(&self, color: Color) -> Square {
        self.pieces(color, PieceKind::King)
            .lsb()
            .expect("every side has a king")
    }

    /// The Zobrist key.
    #[inline]
    pub fn key(&self) -> u64 {
        self.key
    }

    /// The Zobrist key from scratch, to check the incremental one.
    pub fn recompute_key(&self) -> u64 {
        let mut key = 0u64;
        for sq in Square::ALL {
            if let Some(p) = self.mailbox[sq.index()] {
                key ^= zobrist::PIECE[p.color.index()][p.kind.index()][sq.index()];
            }
        }
        key ^= zobrist::CASTLE[self.castling.index()];
        if let Some(ep) = self.en_passant {
            key ^= zobrist::EP[ep.file() as usize];
        }
        if self.side == Color::Black {
            key ^= zobrist::SIDE;
        }
        key
    }

    /// Every piece of either color attacking `sq` with `occ` as the blockers.
    #[inline]
    pub fn attackers_to(&self, sq: Square, occ: Bitboard) -> Bitboard {
        (attacks::pawn_attacks(Color::White, sq) & self.pieces(Color::Black, PieceKind::Pawn))
            | (attacks::pawn_attacks(Color::Black, sq) & self.pieces(Color::White, PieceKind::Pawn))
            | (attacks::knight_attacks(sq) & self.by_kind(PieceKind::Knight))
            | (attacks::king_attacks(sq) & self.by_kind(PieceKind::King))
            | (attacks::rook_attacks(sq, occ)
                & (self.by_kind(PieceKind::Rook) | self.by_kind(PieceKind::Queen)))
            | (attacks::bishop_attacks(sq, occ)
                & (self.by_kind(PieceKind::Bishop) | self.by_kind(PieceKind::Queen)))
    }

    /// Whether any piece of `by` attacks `sq`.
    #[inline]
    pub fn is_attacked(&self, sq: Square, by: Color) -> bool {
        (self.attackers_to(sq, self.occupied()) & self.by_color(by)).is_nonempty()
    }

    /// The pieces giving check to the side to move.
    #[inline]
    pub fn checkers(&self) -> Bitboard {
        self.attackers_to(self.king_square(self.side), self.occupied()) & self.by_color(!self.side)
    }

    /// Whether the side to move is in check.
    #[inline]
    pub fn in_check(&self) -> bool {
        self.checkers().is_nonempty()
    }

    /// Whether `m`, a legal move for the side to move, gives check: a direct
    /// attack from the destination (by the promoted piece, for a promotion),
    /// a discovered attack through the vacated square (and the captured
    /// pawn's square, for en passant), or the rook's attack after castling.
    /// Cheaper than making the move and asking.
    pub fn gives_check(&self, m: Move) -> bool {
        let us = self.side;
        let king = self.king_square(!us);
        let king_bb = Bitboard::from_square(king);
        let from = m.from();
        let to = m.to();
        let mut occ = (self.occupied() & !Bitboard::from_square(from)) | Bitboard::from_square(to);
        if m.is_en_passant() {
            occ &= !Bitboard::from_square(Square::new(to.file(), from.rank()));
        }
        let kind = m
            .promotion()
            .or_else(|| self.piece_at(from).map(|p| p.kind))
            .unwrap_or(PieceKind::Pawn);
        let direct = match kind {
            PieceKind::Pawn => attacks::pawn_attacks(us, to),
            PieceKind::Knight => attacks::knight_attacks(to),
            PieceKind::Bishop => attacks::bishop_attacks(to, occ),
            PieceKind::Rook => attacks::rook_attacks(to, occ),
            PieceKind::Queen => attacks::queen_attacks(to, occ),
            PieceKind::King => Bitboard::EMPTY,
        };
        if (direct & king_bb).is_nonempty() {
            return true;
        }
        // A slider of ours, other than the moved piece, that sees the king
        // now that the vacated squares are empty.
        let ours = self.by_color(us) & !Bitboard::from_square(from);
        let rooks = ours & (self.by_kind(PieceKind::Rook) | self.by_kind(PieceKind::Queen));
        let bishops = ours & (self.by_kind(PieceKind::Bishop) | self.by_kind(PieceKind::Queen));
        if (attacks::rook_attacks(king, occ) & rooks).is_nonempty()
            || (attacks::bishop_attacks(king, occ) & bishops).is_nonempty()
        {
            return true;
        }
        if m.is_castle() {
            let (rook_from, rook_to) = castle_rook_squares(to);
            let occ = (occ & !Bitboard::from_square(rook_from)) | Bitboard::from_square(rook_to);
            return (attacks::rook_attacks(rook_to, occ) & king_bb).is_nonempty();
        }
        false
    }

    /// `sq` if a pawn of the side to move could capture en passant there.
    fn capturable_en_passant(&self, sq: Square) -> Option<Square> {
        let capturers =
            attacks::pawn_attacks(!self.side, sq) & self.pieces(self.side, PieceKind::Pawn);
        if capturers.is_nonempty() {
            Some(sq)
        } else {
            None
        }
    }

    #[inline]
    fn put(&mut self, sq: Square, piece: Piece) {
        debug_assert!(self.mailbox[sq.index()].is_none());
        let bb = Bitboard::from_square(sq);
        self.by_kind[piece.kind.index()] |= bb;
        self.by_color[piece.color.index()] |= bb;
        self.mailbox[sq.index()] = Some(piece);
        self.key ^= zobrist::PIECE[piece.color.index()][piece.kind.index()][sq.index()];
    }

    #[inline]
    fn take(&mut self, sq: Square) -> Piece {
        let piece = self.mailbox[sq.index()].expect("taking from an occupied square");
        let bb = Bitboard::from_square(sq);
        self.by_kind[piece.kind.index()] ^= bb;
        self.by_color[piece.color.index()] ^= bb;
        self.mailbox[sq.index()] = None;
        self.key ^= zobrist::PIECE[piece.color.index()][piece.kind.index()][sq.index()];
        piece
    }

    #[inline]
    fn shift_piece(&mut self, from: Square, to: Square) {
        let piece = self.take(from);
        self.put(to, piece);
    }

    /// Play `m`, which must be legal in this position. Returns what
    /// [`unmake_move`](Self::unmake_move) needs.
    pub fn make_move(&mut self, m: Move) -> Undo {
        let undo = Undo {
            captured: None,
            castling: self.castling,
            en_passant: self.en_passant,
            halfmove: self.halfmove,
            key: self.key,
        };
        let us = self.side;
        let them = !us;
        let (from, to, flag) = (m.from(), m.to(), m.flag());
        let piece = self.mailbox[from.index()].expect("moving from an empty square");
        let mut undo = undo;

        if let Some(ep) = self.en_passant.take() {
            self.key ^= zobrist::EP[ep.file() as usize];
        }
        self.halfmove += 1;

        if m.is_capture() {
            let cap_sq = if flag == MoveFlag::EnPassant {
                to.offset(0, if us == Color::White { -1 } else { 1 })
                    .expect("en passant capture square")
            } else {
                to
            };
            undo.captured = Some(self.take(cap_sq));
            self.halfmove = 0;
        }

        if let Some(kind) = m.promotion() {
            self.take(from);
            self.put(to, Piece::new(us, kind));
        } else {
            self.shift_piece(from, to);
        }

        if piece.kind == PieceKind::Pawn {
            self.halfmove = 0;
            if flag == MoveFlag::DoublePush {
                let ep = from
                    .offset(0, if us == Color::White { 1 } else { -1 })
                    .expect("double push square");
                // Only when an enemy pawn could actually capture there.
                let capturers = attacks::pawn_attacks(us, ep) & self.pieces(them, PieceKind::Pawn);
                if capturers.is_nonempty() {
                    self.en_passant = Some(ep);
                    self.key ^= zobrist::EP[ep.file() as usize];
                }
            }
        } else if m.is_castle() {
            let (rook_from, rook_to) = castle_rook_squares(to);
            self.shift_piece(rook_from, rook_to);
        }

        let lost = CastleRights(CASTLE_MASK[from.index()] | CASTLE_MASK[to.index()]);
        if (self.castling & lost) != CastleRights::NONE {
            self.key ^= zobrist::CASTLE[self.castling.index()];
            self.castling = self.castling & !lost;
            self.key ^= zobrist::CASTLE[self.castling.index()];
        }

        self.side = them;
        self.key ^= zobrist::SIDE;
        if us == Color::Black {
            self.fullmove += 1;
        }
        undo
    }

    /// Pass the turn without moving (for null-move pruning). The en passant
    /// right lapses and the clocks advance as for a quiet move.
    pub fn make_null_move(&mut self) -> NullUndo {
        let undo = NullUndo {
            en_passant: self.en_passant,
            halfmove: self.halfmove,
            key: self.key,
        };
        if let Some(ep) = self.en_passant.take() {
            self.key ^= zobrist::EP[ep.file() as usize];
        }
        self.halfmove += 1;
        if self.side == Color::Black {
            self.fullmove += 1;
        }
        self.side = !self.side;
        self.key ^= zobrist::SIDE;
        undo
    }

    /// Take back a null move.
    pub fn unmake_null_move(&mut self, undo: NullUndo) {
        self.side = !self.side;
        if self.side == Color::Black {
            self.fullmove -= 1;
        }
        self.en_passant = undo.en_passant;
        self.halfmove = undo.halfmove;
        self.key = undo.key;
    }

    /// Whether `color` has any piece besides pawns and the king. Null-move
    /// pruning is unsafe without one: zugzwang is common in pawn endings.
    #[inline]
    pub fn has_non_pawn_material(&self, color: Color) -> bool {
        (self.by_color(color) & !(self.by_kind(PieceKind::Pawn) | self.by_kind(PieceKind::King)))
            .is_nonempty()
    }

    /// Take back `m`, the last move played, with the `Undo` it returned.
    pub fn unmake_move(&mut self, m: Move, undo: Undo) {
        let them = self.side;
        let us = !them;
        let (from, to, flag) = (m.from(), m.to(), m.flag());

        self.side = us;
        if us == Color::Black {
            self.fullmove -= 1;
        }

        if m.is_promotion() {
            self.take(to);
            self.put(from, Piece::new(us, PieceKind::Pawn));
        } else {
            self.shift_piece(to, from);
        }
        if m.is_castle() {
            let (rook_from, rook_to) = castle_rook_squares(to);
            self.shift_piece(rook_to, rook_from);
        }
        if let Some(captured) = undo.captured {
            let cap_sq = if flag == MoveFlag::EnPassant {
                to.offset(0, if us == Color::White { -1 } else { 1 })
                    .expect("en passant capture square")
            } else {
                to
            };
            self.put(cap_sq, captured);
        }

        self.castling = undo.castling;
        self.en_passant = undo.en_passant;
        self.halfmove = undo.halfmove;
        self.key = undo.key;
    }
}

/// Where the rook goes when the king lands on `king_to`.
#[inline]
fn castle_rook_squares(king_to: Square) -> (Square, Square) {
    match king_to {
        Square::G1 => (Square::H1, Square::F1),
        Square::C1 => (Square::A1, Square::D1),
        Square::G8 => (Square::H8, Square::F8),
        Square::C8 => (Square::A8, Square::D8),
        _ => unreachable!("castling king destination {king_to}"),
    }
}

impl fmt::Debug for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Position({})", self.to_fen())
    }
}

impl fmt::Display for Position {
    /// An ASCII board from white's side, then the FEN.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            for file in 0..8 {
                let c = self.mailbox[Square::new(file, rank).index()].map_or('.', Piece::to_char);
                write!(f, "{c}{}", if file < 7 { " " } else { "" })?;
            }
            writeln!(f)?;
        }
        write!(f, "{}", self.to_fen())
    }
}
