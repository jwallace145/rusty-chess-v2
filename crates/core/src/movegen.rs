//! Legal move generation and perft.
//!
//! Moves are generated legal, not pseudo-legal: the king's attackers give a
//! *check mask* (the squares a non-king move may go to), pinned pieces are
//! found by scanning the enemy sliders aligned with the king and may only
//! move along their pin line, and king moves are checked against attacks
//! with the king removed from the occupancy. En passant is the one case
//! that is verified by hand, because the capture removes two pieces from a
//! rank at once.

use crate::attacks;
use crate::bitboard::Bitboard;
use crate::r#move::{Move, MoveFlag};
use crate::position::Position;
use crate::types::{Color, PieceKind, Square};

/// The most moves any legal position can have is 218; 256 leaves room.
pub const MAX_MOVES: usize = 256;

/// A stack-allocated list of moves.
#[derive(Clone)]
pub struct MoveList {
    moves: [Move; MAX_MOVES],
    len: usize,
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl MoveList {
    /// An empty list.
    #[inline]
    pub const fn new() -> MoveList {
        MoveList {
            moves: [Move::NULL; MAX_MOVES],
            len: 0,
        }
    }

    /// Append a move.
    #[inline]
    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES);
        self.moves[self.len] = m;
        self.len += 1;
    }

    /// How many moves.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether there are none.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Forget every move.
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The moves.
    #[inline]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }

    /// The moves, mutably (for ordering).
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [Move] {
        &mut self.moves[..self.len]
    }

    /// Iterate the moves.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, Move> {
        self.as_slice().iter()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = std::slice::Iter<'a, Move>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Position {
    /// Append every legal move to `list`.
    pub fn legal_moves(&self, list: &mut MoveList) {
        generate(self, list);
    }

    /// The move in UCI notation (`e2e4`, `e7e8q`) if it is legal here.
    pub fn parse_uci_move(&self, text: &str) -> Option<Move> {
        let mut list = MoveList::new();
        self.legal_moves(&mut list);
        list.iter().copied().find(|m| m.to_string() == text)
    }
}

/// Generate every legal move for the side to move.
pub fn generate(pos: &Position, list: &mut MoveList) {
    let us = pos.side_to_move();
    let them = !us;
    let ours = pos.by_color(us);
    let theirs = pos.by_color(them);
    let occ = ours | theirs;
    let king = pos.king_square(us);
    let king_bb = Bitboard::from_square(king);

    // King moves: the king must not step onto an attacked square, with
    // itself removed so that sliders x-ray through it.
    let occ_no_king = occ ^ king_bb;
    for to in attacks::king_attacks(king) & !ours {
        if (pos.attackers_to(to, occ_no_king) & theirs).is_empty() {
            list.push(Move::new(
                king,
                to,
                if theirs.contains(to) {
                    MoveFlag::Capture
                } else {
                    MoveFlag::Quiet
                },
            ));
        }
    }

    let checkers = pos.attackers_to(king, occ) & theirs;
    if checkers.more_than_one() {
        return; // only the king can move out of a double check
    }
    let check_mask = match checkers.lsb() {
        Some(checker) => attacks::between(king, checker) | checkers,
        None => Bitboard::ALL,
    };

    // Pinned pieces: an enemy slider aligned with the king with exactly one
    // of our pieces between.
    let their_rooks = pos.pieces(them, PieceKind::Rook) | pos.pieces(them, PieceKind::Queen);
    let their_bishops = pos.pieces(them, PieceKind::Bishop) | pos.pieces(them, PieceKind::Queen);
    let mut pinned = Bitboard::EMPTY;
    let snipers = (attacks::rook_attacks(king, theirs) & their_rooks)
        | (attacks::bishop_attacks(king, theirs) & their_bishops);
    for sniper in snipers {
        let blockers = attacks::between(king, sniper) & occ;
        if blockers.is_nonempty() && !blockers.more_than_one() && (blockers & ours).is_nonempty() {
            pinned |= blockers;
        }
    }
    let targets = !ours & check_mask;

    // Knights: a pinned knight can never move.
    for from in pos.pieces(us, PieceKind::Knight) & !pinned {
        push_all(list, from, attacks::knight_attacks(from) & targets, theirs);
    }

    // Sliders: a pinned slider stays on its pin line.
    let bishops = pos.pieces(us, PieceKind::Bishop) | pos.pieces(us, PieceKind::Queen);
    for from in bishops {
        let mut to = attacks::bishop_attacks(from, occ) & targets;
        if pinned.contains(from) {
            to &= attacks::line(king, from);
        }
        push_all(list, from, to, theirs);
    }
    let rooks = pos.pieces(us, PieceKind::Rook) | pos.pieces(us, PieceKind::Queen);
    for from in rooks {
        let mut to = attacks::rook_attacks(from, occ) & targets;
        if pinned.contains(from) {
            to &= attacks::line(king, from);
        }
        push_all(list, from, to, theirs);
    }

    // Pawns.
    let pawns = pos.pieces(us, PieceKind::Pawn);
    let (promo_rank, double_rank, up): (Bitboard, Bitboard, i8) = match us {
        Color::White => (Bitboard::RANK_8, Bitboard::RANK_3, 1),
        Color::Black => (Bitboard::RANK_1, Bitboard::RANK_6, -1),
    };
    let single = pawns.shift(us) & !occ;
    let double = (single & double_rank).shift(us) & !occ;
    for to in single & check_mask {
        let from = to.offset(0, -up).expect("pawn push origin");
        if pinned.contains(from) && !attacks::line(king, from).contains(to) {
            continue;
        }
        if promo_rank.contains(to) {
            push_promotions(list, from, to, false);
        } else {
            list.push(Move::new(from, to, MoveFlag::Quiet));
        }
    }
    for to in double & check_mask {
        let from = to.offset(0, -2 * up).expect("double push origin");
        if pinned.contains(from) && !attacks::line(king, from).contains(to) {
            continue;
        }
        list.push(Move::new(from, to, MoveFlag::DoublePush));
    }
    for from in pawns {
        let mut captures = attacks::pawn_attacks(us, from) & theirs & check_mask;
        if pinned.contains(from) {
            captures &= attacks::line(king, from);
        }
        for to in captures {
            if promo_rank.contains(to) {
                push_promotions(list, from, to, true);
            } else {
                list.push(Move::new(from, to, MoveFlag::Capture));
            }
        }
    }
    if let Some(ep) = pos.en_passant() {
        let captured = ep.offset(0, -up).expect("en passant captured pawn");
        for from in attacks::pawn_attacks(them, ep) & pawns {
            // Verify by hand: remove both pawns, add ours on the target, and
            // see whether the king is attacked (covers pins, the rank x-ray
            // and the case where the captured pawn is the checker).
            let occ_after =
                (occ ^ Bitboard::from_square(from) ^ Bitboard::from_square(captured)).with(ep);
            let attackers = (attacks::rook_attacks(king, occ_after) & their_rooks)
                | (attacks::bishop_attacks(king, occ_after) & their_bishops)
                | (attacks::knight_attacks(king) & pos.pieces(them, PieceKind::Knight))
                | (attacks::pawn_attacks(us, king)
                    & pos.pieces(them, PieceKind::Pawn).without(captured));
            if attackers.is_empty() {
                list.push(Move::new(from, ep, MoveFlag::EnPassant));
            }
        }
    }

    // Castling: not in check, the squares between king and rook empty, and
    // the king's path not attacked.
    if checkers.is_empty() {
        let rights = pos.castling();
        let (king_side, queen_side, rank) = match us {
            Color::White => (
                crate::position::CastleRights::WHITE_KING,
                crate::position::CastleRights::WHITE_QUEEN,
                0,
            ),
            Color::Black => (
                crate::position::CastleRights::BLACK_KING,
                crate::position::CastleRights::BLACK_QUEEN,
                7,
            ),
        };
        let f = Square::new(5, rank);
        let g = Square::new(6, rank);
        if rights.contains(king_side)
            && (occ & (Bitboard::from_square(f) | Bitboard::from_square(g))).is_empty()
            && (pos.attackers_to(f, occ) & theirs).is_empty()
            && (pos.attackers_to(g, occ) & theirs).is_empty()
        {
            list.push(Move::new(king, g, MoveFlag::KingCastle));
        }
        let b = Square::new(1, rank);
        let c = Square::new(2, rank);
        let d = Square::new(3, rank);
        if rights.contains(queen_side)
            && (occ
                & (Bitboard::from_square(b) | Bitboard::from_square(c) | Bitboard::from_square(d)))
            .is_empty()
            && (pos.attackers_to(c, occ) & theirs).is_empty()
            && (pos.attackers_to(d, occ) & theirs).is_empty()
        {
            list.push(Move::new(king, c, MoveFlag::QueenCastle));
        }
    }
}

#[inline]
fn push_all(list: &mut MoveList, from: Square, targets: Bitboard, theirs: Bitboard) {
    for to in targets & theirs {
        list.push(Move::new(from, to, MoveFlag::Capture));
    }
    for to in targets & !theirs {
        list.push(Move::new(from, to, MoveFlag::Quiet));
    }
}

#[inline]
fn push_promotions(list: &mut MoveList, from: Square, to: Square, capture: bool) {
    for kind in [
        PieceKind::Queen,
        PieceKind::Rook,
        PieceKind::Bishop,
        PieceKind::Knight,
    ] {
        list.push(Move::new(from, to, MoveFlag::promotion(kind, capture)));
    }
}

/// Leaf nodes at `depth`, counting the last ply in bulk (the move count,
/// without making the moves).
pub fn perft(pos: &mut Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut list = MoveList::new();
    generate(pos, &mut list);
    if depth == 1 {
        return list.len() as u64;
    }
    let mut nodes = 0;
    for &m in list.iter() {
        let undo = pos.make_move(m);
        nodes += perft(pos, depth - 1);
        pos.unmake_move(m, undo);
    }
    nodes
}

/// Perft split by root move, for finding where a count goes wrong.
pub fn perft_divide(pos: &mut Position, depth: u32) -> Vec<(Move, u64)> {
    let mut list = MoveList::new();
    generate(pos, &mut list);
    let mut out = Vec::with_capacity(list.len());
    for &m in list.iter() {
        let nodes = if depth <= 1 {
            1
        } else {
            let undo = pos.make_move(m);
            let n = perft(pos, depth - 1);
            pos.unmake_move(m, undo);
            n
        };
        out.push((m, nodes));
    }
    out
}
