//! The search's view of a position: the [`Position`] itself, the evaluation
//! terms kept up to date through make and unmake, and the Zobrist keys of
//! the positions on the way here for repetition detection. With a network
//! set, also a stack of NNUE accumulators, one entry per ply, pushed by make
//! and popped by unmake. Nothing here allocates after construction.

use rusty_chess_core::r#move::{Move, MoveFlag};
use rusty_chess_core::position::{NullUndo, Position, Undo};
use rusty_chess_core::types::{Color, PieceKind, Square};

use std::sync::Arc;

use crate::eval::Eval;
use crate::nnue::{Accumulator, Network, feature};
use crate::stats::MAX_PLY;

/// Game history plus search depth; a game of 400 moves is 800 keys.
const MAX_HISTORY: usize = 2048;

/// NNUE accumulator entries: a search never goes deeper than `MAX_PLY`
/// from its root, where the stack is compacted (`set_root`).
const NNUE_STACK: usize = MAX_PLY + 8;

/// A position with its evaluation and key history.
#[derive(Clone)]
pub struct Board {
    pos: Position,
    eval: Eval,
    keys: [u64; MAX_HISTORY],
    len: usize,
    root: usize,
    nnue: Option<Nnue>,
}

/// The network and the accumulators of the current line.
#[derive(Clone)]
struct Nnue {
    net: Arc<Network>,
    /// `[white's perspective, black's]` per ply; `top` is the current one.
    stack: Box<[[Accumulator; 2]]>,
    top: usize,
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
            nnue: None,
        }
    }

    /// Evaluate with `net` from now on (the accumulators are computed from
    /// scratch), or with the hand-written evaluation when `None`.
    pub fn set_network(&mut self, net: Option<Arc<Network>>) {
        self.nnue = net.map(|net| {
            let mut stack =
                vec![[Accumulator([0; crate::nnue::HIDDEN]); 2]; NNUE_STACK].into_boxed_slice();
            stack[0] = [
                net.refresh(&self.pos, Color::White),
                net.refresh(&self.pos, Color::Black),
            ];
            Nnue { net, stack, top: 0 }
        });
    }

    /// Whether a network evaluates this board.
    pub fn has_network(&self) -> bool {
        self.nnue.is_some()
    }

    /// The current accumulators, `[white's perspective, black's]`, when a
    /// network is set (for tests).
    pub fn accumulators(&self) -> Option<[Accumulator; 2]> {
        self.nnue.as_ref().map(|n| n.stack[n.top])
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

    /// The evaluation from the side to move's point of view: the network's
    /// when one is set, the tapered hand-written one otherwise.
    #[inline]
    pub fn evaluate(&self) -> i32 {
        if let Some(n) = &self.nnue {
            let [w, b] = &n.stack[n.top];
            return if self.pos.side_to_move() == Color::White {
                n.net.evaluate(w, b)
            } else {
                n.net.evaluate(b, w)
            };
        }
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
        // The search starts from the bottom of the accumulator stack, so it
        // can go MAX_PLY deep whatever the game's length.
        if let Some(n) = &mut self.nnue {
            n.stack[0] = n.stack[n.top];
            n.top = 0;
        }
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
        let placed = m.promotion().unwrap_or(piece.kind);
        let captured = undo.captured().map(|c| {
            let sq = if flag == MoveFlag::EnPassant {
                to.offset(0, if us == Color::White { -1 } else { 1 })
                    .expect("en passant capture square")
            } else {
                to
            };
            (c, sq)
        });
        let castle = m.is_castle().then(|| rook_squares(to));

        if let Some((c, sq)) = captured {
            self.eval.remove(c.color, c.kind, sq);
        }
        self.eval.remove(us, piece.kind, from);
        self.eval.add(us, placed, to);
        if let Some((rook_from, rook_to)) = castle {
            self.eval.remove(us, PieceKind::Rook, rook_from);
            self.eval.add(us, PieceKind::Rook, rook_to);
        }

        if let Some(n) = &mut self.nnue {
            // Outside a search (a long game played move by move) the stack
            // can fill; it then restarts from the bottom, which is safe
            // because a game is never unmade there.
            if n.top + 1 == n.stack.len() {
                n.stack[0] = n.stack[n.top];
                n.top = 0;
            }
            // The next ply's accumulators, from the current ones in one pass
            // per perspective.
            let (below, above) = n.stack.split_at_mut(n.top + 1);
            let (old, new) = (&below[n.top], &mut above[0]);
            for (i, view) in [Color::White, Color::Black].into_iter().enumerate() {
                let on = feature(view, us, placed, to);
                let off = feature(view, us, piece.kind, from);
                match (captured, castle) {
                    (Some((c, sq)), _) => new[i].add_sub_sub(
                        &old[i],
                        &n.net,
                        on,
                        off,
                        feature(view, c.color, c.kind, sq),
                    ),
                    (None, Some((rook_from, rook_to))) => new[i].add_add_sub_sub(
                        &old[i],
                        &n.net,
                        [on, feature(view, us, PieceKind::Rook, rook_to)],
                        [off, feature(view, us, PieceKind::Rook, rook_from)],
                    ),
                    (None, None) => new[i].add_sub(&old[i], &n.net, on, off),
                }
            }
            n.top += 1;
        }

        debug_assert!(self.len < MAX_HISTORY, "game too long for the key history");
        self.keys[self.len] = self.pos.key();
        self.len += 1;
        undo
    }

    /// Pass the turn (null-move pruning); the evaluation terms are unchanged.
    pub fn make_null_move(&mut self) -> NullUndo {
        let undo = self.pos.make_null_move();
        debug_assert!(self.len < MAX_HISTORY, "game too long for the key history");
        self.keys[self.len] = self.pos.key();
        self.len += 1;
        undo
    }

    /// Take back a null move.
    pub fn unmake_null_move(&mut self, undo: NullUndo) {
        self.len -= 1;
        self.pos.unmake_null_move(undo);
    }

    /// Take back `m`.
    pub fn unmake_move(&mut self, m: Move, undo: Undo) {
        self.len -= 1;
        if let Some(n) = &mut self.nnue {
            debug_assert!(n.top > 0, "unmake past the accumulator stack's bottom");
            n.top -= 1;
        }
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
