//! Engine against engine through the library: every move the search returns
//! must be legal, the game must end in a real terminal state or the ply cap,
//! and nothing may panic. This is the in-repo stand-in for the fastchess
//! games the roadmap asks for; fastchess measures strength, this checks
//! legality.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{Limits, Searcher};

fn play(fen: &str, depth: u32, max_plies: usize) -> (usize, &'static str) {
    let mut board = Board::from_position(Position::from_fen(fen).unwrap());
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    let limits = Limits {
        depth: Some(depth),
        ..Limits::default()
    };
    for ply in 0..max_plies {
        let mut legal = MoveList::new();
        board.position().legal_moves(&mut legal);
        if legal.is_empty() {
            return (
                ply,
                if board.position().in_check() {
                    "checkmate"
                } else {
                    "stalemate"
                },
            );
        }
        if board.is_repetition() || board.is_fifty_moves() {
            return (ply, "draw");
        }
        let result = searcher.search(&mut board, &limits, &mut |_| {});
        assert!(
            legal.iter().any(|&m| m == result.best),
            "{} is not legal in {}",
            result.best,
            board.position().to_fen()
        );
        assert_eq!(
            board.ply_from_root(),
            0,
            "the search left moves on the board"
        );
        board.make_move(result.best);
    }
    (max_plies, "ply cap")
}

#[test]
fn self_play_from_the_start_position_is_legal_throughout() {
    let (plies, end) = play(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        2,
        200,
    );
    assert!(plies > 10, "{plies} plies, {end}");
}

#[test]
fn self_play_from_tactical_and_endgame_positions() {
    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "4k3/8/8/8/8/8/4P3/4K3 w - - 0 1",
        "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1",
    ] {
        let (plies, end) = play(fen, 2, 150);
        assert!(plies > 0, "{fen}: {plies} plies, {end}");
    }
}
