//! PeSTO tapered evaluation and the incrementally updated search board.

use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};
use rusty_chess_search::board::Board;
use rusty_chess_search::eval::{self, Eval};

fn board(fen: &str) -> Board {
    Board::from_position(Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}")))
}

/// Mirror a FEN top to bottom with colors and the side to move swapped. The
/// evaluation of the mirror must be identical, from the mover's view.
fn mirror(fen: &str) -> String {
    let fields: Vec<&str> = fen.split(' ').collect();
    let ranks: Vec<String> = fields[0]
        .split('/')
        .rev()
        .map(|r| {
            r.chars()
                .map(|c| {
                    if c.is_ascii_alphabetic() {
                        if c.is_ascii_uppercase() {
                            c.to_ascii_lowercase()
                        } else {
                            c.to_ascii_uppercase()
                        }
                    } else {
                        c
                    }
                })
                .collect()
        })
        .collect();
    let side = if fields[1] == "w" { "b" } else { "w" };
    let castling: String = if fields[2] == "-" {
        "-".into()
    } else {
        let mut s: Vec<char> = fields[2]
            .chars()
            .map(|c| {
                if c.is_ascii_uppercase() {
                    c.to_ascii_lowercase()
                } else {
                    c.to_ascii_uppercase()
                }
            })
            .collect();
        s.sort_by_key(|c| match c {
            'K' => 0,
            'Q' => 1,
            'k' => 2,
            'q' => 3,
            _ => 4,
        });
        s.into_iter().collect()
    };
    let ep = if fields[3] == "-" {
        "-".to_string()
    } else {
        let b = fields[3].as_bytes();
        format!("{}{}", b[0] as char, (b'9' - b[1] + b'0') as char)
    };
    format!(
        "{} {side} {castling} {ep} {} {}",
        ranks.join("/"),
        fields[4],
        fields[5]
    )
}

#[test]
fn piece_values_and_phase_are_pesto() {
    assert_eq!(eval::MG_VALUE, [82, 337, 365, 477, 1025, 0]);
    assert_eq!(eval::EG_VALUE, [94, 281, 297, 512, 936, 0]);
    assert_eq!(eval::PHASE_INC, [0, 1, 1, 2, 4, 0]);
    assert_eq!(eval::PHASE_TOTAL, 24);
    // A white pawn on e4 scores the PeSTO table entry; the same for black on e5 from black's view.
    let (mg_w, _) = eval::psqt(Color::White, PieceKind::Pawn, "e4".parse().unwrap());
    let (mg_b, _) = eval::psqt(Color::Black, PieceKind::Pawn, "e5".parse().unwrap());
    assert_eq!(mg_w, 82 + 17);
    assert_eq!(mg_b, 82 + 17);
    assert_eq!(
        eval::psqt(Color::White, PieceKind::Knight, "a1".parse().unwrap()).0,
        337 - 105
    );
}

#[test]
fn the_start_position_is_balanced_and_material_counts() {
    assert_eq!(
        board("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").evaluate(),
        0
    );
    let up_a_queen = board("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").evaluate();
    assert!(up_a_queen > 800, "{up_a_queen}");
    let black_to_move =
        board("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1").evaluate();
    assert_eq!(black_to_move, -up_a_queen);
}

#[test]
fn evaluation_is_symmetric_under_mirroring() {
    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        "6k1/4pp1p/3p2p1/P1pPb3/R7/1r2P1PP/3B1P2/6K1 w - - 0 1",
    ] {
        let m = mirror(fen);
        assert_eq!(board(fen).evaluate(), board(&m).evaluate(), "{fen} vs {m}");
    }
}

#[test]
fn tapering_moves_from_middlegame_to_endgame_values() {
    // King on g1 and h8 only: a pure endgame, scored with the endgame king table.
    let e = board("7k/8/8/8/8/8/8/6K1 w - - 0 1").eval_state();
    assert_eq!(e.phase, 0);
    assert_eq!(
        e.mg,
        eval::psqt(Color::White, PieceKind::King, "g1".parse().unwrap()).0
            - eval::psqt(Color::Black, PieceKind::King, "h8".parse().unwrap()).0
    );
    let full = board("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").eval_state();
    assert_eq!(full.phase, 24);
    let promoted = board("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNQ w KQkq - 0 1").eval_state();
    assert_eq!(
        promoted.phase, 26,
        "phase can exceed the total before clamping"
    );
    assert_eq!(
        Eval {
            mg: 100,
            eg: 0,
            phase: 26
        }
        .taper(),
        100
    );
    assert_eq!(
        Eval {
            mg: 100,
            eg: 0,
            phase: 12
        }
        .taper(),
        50
    );
    assert_eq!(
        Eval {
            mg: 0,
            eg: 100,
            phase: 0
        }
        .taper(),
        100
    );
}

#[test]
fn incremental_evaluation_matches_a_full_recompute_along_random_games() {
    let mut state = 0x1234_5678_9ABC_DEF0u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ] {
        for _ in 0..20 {
            let mut b = board(fen);
            let start = b.clone();
            let mut played = Vec::new();
            for _ in 0..40 {
                let mut list = MoveList::new();
                b.position().legal_moves(&mut list);
                if list.is_empty() {
                    break;
                }
                let m = list.as_slice()[(next() % list.len() as u64) as usize];
                let undo = b.make_move(m);
                played.push((m, undo));
                assert_eq!(
                    b.eval_state(),
                    Board::from_position(b.position().clone()).eval_state(),
                    "after {m} from {}",
                    b.position().to_fen()
                );
            }
            for (m, undo) in played.into_iter().rev() {
                b.unmake_move(m, undo);
            }
            assert_eq!(b.eval_state(), start.eval_state());
            assert_eq!(b.position(), start.position());
        }
    }
}

#[test]
fn the_board_tracks_repetitions_and_the_fifty_move_rule() {
    let mut b = board("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let shuffle = ["g1f3", "g8f6", "f3g1", "f6g8"];
    assert!(!b.is_repetition());
    for _ in 0..2 {
        for uci in shuffle {
            let m = b.position().parse_uci_move(uci).unwrap();
            b.make_move(m);
        }
    }
    // The start position has now occurred three times (the initial one plus two returns).
    assert!(b.is_repetition());
    assert_eq!(b.ply_from_root(), 8);
    assert!(!b.is_fifty_moves());
    let mut b = board("8/8/8/8/8/8/8/K6k w - - 99 80");
    assert!(!b.is_fifty_moves());
    b.make_move(b.position().parse_uci_move("a1a2").unwrap());
    assert!(b.is_fifty_moves());
}
