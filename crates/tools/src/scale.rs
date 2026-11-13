//! What an evaluation thinks a piece is worth, in its own units.
//!
//! The search's margins (reverse futility, futility, the aspiration window)
//! are written in the evaluation's units, and a net's units are whatever its
//! training made them: a net trained on win probability at a scale of 400
//! values a pawn at far more than the 100 a hand-written evaluation does, and
//! the figure moves from net to net. This module measures it: for each
//! position, every piece but the kings is removed in turn and the static
//! evaluation read again; the change, signed for the piece's owner, is what
//! that piece was worth there.
//!
//! The same measurement on PeSTO returns its table values (a pawn near 80, a
//! queen near 1,000), which is the check that it measures what it says.

use std::sync::Arc;

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{PieceKind, Square};
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::Network;

/// The kinds measured, in [`PieceKind`] order: every piece but the king.
pub const KINDS: [PieceKind; 5] = [
    PieceKind::Pawn,
    PieceKind::Knight,
    PieceKind::Bishop,
    PieceKind::Rook,
    PieceKind::Queen,
];

/// A kind's values over a set of positions.
#[derive(Clone, Debug, PartialEq)]
pub struct Summary {
    /// How many pieces of the kind were measured.
    pub samples: usize,
    /// The mean value.
    pub mean: f64,
    /// The 10th percentile.
    pub p10: i32,
    /// The median.
    pub median: i32,
    /// The 90th percentile.
    pub p90: i32,
}

/// `fen` with the piece on `square` taken off the board, or `None` when the
/// square is empty or the FEN's placement cannot be read. Castling rights and
/// the en passant square are dropped (a removed rook or pawn would leave them
/// wrong), and the move counters reset: the result is for evaluating, not for
/// playing on from.
pub fn without_piece(fen: &str, square: Square) -> Option<String> {
    let mut fields = fen.split_whitespace();
    let placement = fields.next()?;
    let side = fields.next()?;
    let mut rows: Vec<Vec<char>> = Vec::with_capacity(8);
    for row in placement.split('/') {
        let mut cells = Vec::with_capacity(8);
        for c in row.chars() {
            match c.to_digit(10) {
                Some(n) => cells.extend(std::iter::repeat_n('.', n as usize)),
                None => cells.push(c),
            }
        }
        if cells.len() != 8 {
            return None;
        }
        rows.push(cells);
    }
    if rows.len() != 8 {
        return None;
    }
    let (file, rank) = (square.index() % 8, square.index() / 8);
    let cell = &mut rows[7 - rank][file];
    if *cell == '.' {
        return None;
    }
    *cell = '.';
    let placement: Vec<String> = rows
        .iter()
        .map(|row| {
            let mut out = String::new();
            let mut run = 0;
            for &c in row {
                if c == '.' {
                    run += 1;
                } else {
                    if run > 0 {
                        out.push_str(&run.to_string());
                        run = 0;
                    }
                    out.push(c);
                }
            }
            if run > 0 {
                out.push_str(&run.to_string());
            }
            out
        })
        .collect();
    Some(format!("{} {side} - - 0 1", placement.join("/")))
}

/// The static evaluation of `fen` for the side to move, with `net` or, when
/// `None`, the hand-written evaluation; `None` for a FEN the engine does not
/// read or a position in check, where the evaluation means nothing.
pub fn evaluate(fen: &str, net: Option<&Arc<Network>>) -> Option<i32> {
    let pos = Position::from_fen(fen).ok()?;
    if pos.in_check() {
        return None;
    }
    let mut board = Board::from_position(pos);
    board.set_network(net.cloned());
    Some(board.evaluate())
}

/// Every non-king piece's value in every position of `fens`, by kind (the
/// order of [`KINDS`]): the evaluation with the piece less the evaluation
/// without it, from its owner's point of view. Positions the engine does not
/// read, positions in check, and removals that leave the side to move giving
/// check are skipped.
pub fn piece_values<'a>(
    fens: impl IntoIterator<Item = &'a str>,
    net: Option<&Arc<Network>>,
) -> [Vec<i32>; 5] {
    let mut values: [Vec<i32>; 5] = Default::default();
    for fen in fens {
        let Ok(pos) = Position::from_fen(fen) else {
            continue;
        };
        let Some(with) = evaluate(fen, net) else {
            continue;
        };
        let side = pos.side_to_move();
        for square in Square::ALL {
            let Some(piece) = pos.piece_at(square) else {
                continue;
            };
            let Some(slot) = KINDS.iter().position(|&k| k == piece.kind) else {
                continue;
            };
            let Some(without) = without_piece(fen, square).and_then(|f| evaluate(&f, net)) else {
                continue;
            };
            // The side to move's evaluation falls when its own piece goes
            // and rises when the opponent's does.
            values[slot].push(if piece.color == side {
                with - without
            } else {
                without - with
            });
        }
    }
    values
}

/// The mean and three percentiles of `values`, or `None` when it is empty.
pub fn summarize(values: &[i32]) -> Option<Summary> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let at = |q: f64| sorted[((sorted.len() - 1) as f64 * q) as usize];
    Some(Summary {
        samples: sorted.len(),
        mean: sorted.iter().map(|&v| f64::from(v)).sum::<f64>() / sorted.len() as f64,
        p10: at(0.1),
        median: at(0.5),
        p90: at(0.9),
    })
}
