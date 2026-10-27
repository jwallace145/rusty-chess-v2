//! The search's view of a position: the [`Position`] itself, the evaluation
//! terms kept up to date through make and unmake, and the Zobrist keys of
//! the positions on the way here for repetition detection. Nothing here
//! allocates after construction.

use rusty_chess_core::r#move::{Move, MoveFlag};
use rusty_chess_core::position::{Position, Undo};
use rusty_chess_core::types::{Color, PieceKind, Square};

use crate::eval::Eval;

/// Game history plus search depth; a game of 400 moves is 800 keys.
const MAX_HISTORY: usize = 2048;

/// A position with its evaluation and key history.
#[derive(Clone)]
pub struct Board {
    pos: Position,
    eval: Eval,
    keys: [u64; MAX_HISTORY],
    len: usize,
    root: usize,
}

impl Board {
    /// Wrap a position; its key starts the history.
    pub fn from_position(pos: Position) -> Board {
        let mut eval = Eval::default();
        for sq in Square::ALL {
            if let Some(p) = pos.piece_at(sq) {
                eval.add(p.color, p.kind, sq);
            }
        }
        let mut keys = [0u64; MAX_HISTORY];
        keys[0] = pos.key();
        Board {
            pos,
            eval,
            keys,
            len: 1,
            root: 0,
        }
    }

    /// The position.
    #[inline]
    pub fn position(&self) -> &Position {
        &self.pos
    }

    /// The untapered evaluation terms, from white's point of view.
    #[inline]
    pub fn eval_state(&self) -> Eval {
        self.eval
    }

    /// The tapered evaluation from the side to move's point of view.
    #[inline]
    pub fn evaluate(&self) -> i32 {
        let white = self.eval.taper();
        if self.pos.side_to_move() == Color::White {
            white
        } else {
            -white
        }
    }

    /// Mark the current position as the search root: plies are counted from
    /// here, and repetitions before it still count.
    #[inline]
    pub fn set_root(&mut self) {
        self.root = self.len - 1;
    }

    /// Plies played since the root.
    #[inline]
    pub fn ply_from_root(&self) -> usize {
        self.len - 1 - self.root
    }

    /// Whether the current position occurred before, within the plies the
    /// fifty-move counter spans. A single earlier occurrence counts: in a
    /// search, repeating once is enough to claim the draw.
    pub fn is_repetition(&self) -> bool {
        let key = self.pos.key();
        let span = (self.pos.halfmove_clock() as usize).min(self.len - 1);
        let mut i = self.len - 1;
        let mut back = 2;
        while back <= span {
            i -= 2;
            if self.keys[i] == key {
                return true;
            }
            back += 2;
        }
        false
    }

    /// Whether fifty moves have passed without a capture or pawn move.
    #[inline]
    pub fn is_fifty_moves(&self) -> bool {
        self.pos.halfmove_clock() >= 100
    }

    /// Play `m`, updating the evaluation and the history.
    pub fn make_move(&mut self, m: Move) -> Undo {
        let us = self.pos.side_to_move();
        let (from, to, flag) = (m.from(), m.to(), m.flag());
        let piece = self
            .pos
            .piece_at(from)
            .expect("moving from an empty square");
        let undo = self.pos.make_move(m);

        if let Some(captured) = undo.captured() {
            let cap_sq = if flag == MoveFlag::EnPassant {
                to.offset(0, if us == Color::White { -1 } else { 1 })
                    .expect("en passant capture square")
            } else {
                to
            };
            self.eval.remove(captured.color, captured.kind, cap_sq);
        }
        self.eval.remove(us, piece.kind, from);
        self.eval.add(us, m.promotion().unwrap_or(piece.kind), to);
        if m.is_castle() {
            let (rook_from, rook_to) = rook_squares(to);
            self.eval.remove(us, PieceKind::Rook, rook_from);
            self.eval.add(us, PieceKind::Rook, rook_to);
        }

        debug_assert!(self.len < MAX_HISTORY, "game too long for the key history");
        self.keys[self.len] = self.pos.key();
        self.len += 1;
        undo
    }

    /// Take back `m`.
    pub fn unmake_move(&mut self, m: Move, undo: Undo) {
        self.len -= 1;
        self.pos.unmake_move(m, undo);
        let us = self.pos.side_to_move();
        let (from, to, flag) = (m.from(), m.to(), m.flag());
        let piece = self
            .pos
            .piece_at(from)
            .expect("unmade piece is back on its square");

        if m.is_castle() {
            let (rook_from, rook_to) = rook_squares(to);
            self.eval.remove(us, PieceKind::Rook, rook_to);
            self.eval.add(us, PieceKind::Rook, rook_from);
        }
        self.eval
            .remove(us, m.promotion().unwrap_or(piece.kind), to);
        self.eval.add(us, piece.kind, from);
        if let Some(captured) = undo.captured() {
            let cap_sq = if flag == MoveFlag::EnPassant {
                to.offset(0, if us == Color::White { -1 } else { 1 })
                    .expect("en passant capture square")
            } else {
                to
            };
            self.eval.add(captured.color, captured.kind, cap_sq);
        }
    }
}

#[inline]
fn rook_squares(king_to: Square) -> (Square, Square) {
    match king_to {
        Square::G1 => (Square::H1, Square::F1),
        Square::C1 => (Square::A1, Square::D1),
        Square::G8 => (Square::H8, Square::F8),
        Square::C8 => (Square::A8, Square::D8),
        _ => unreachable!("castling king destination {king_to}"),
    }
}
