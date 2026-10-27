//! Legal move generation and perft on positions with known answers. The full
//! suite runs from the root crate's `tests/perft.rs`.

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::{MoveList, perft, perft_divide};
use rusty_chess_core::position::Position;

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"))
}

fn moves(fen: &str) -> Vec<String> {
    let p = pos(fen);
    let mut list = MoveList::new();
    p.legal_moves(&mut list);
    let mut v: Vec<String> = list.iter().map(|m| m.to_string()).collect();
    v.sort();
    v
}

#[test]
fn move_list_is_a_fixed_array() {
    let mut list = MoveList::new();
    assert!(list.is_empty());
    list.push(Move::NULL);
    assert_eq!(list.len(), 1);
    assert_eq!(list.as_slice(), &[Move::NULL]);
    list.clear();
    assert!(list.is_empty());
    assert!(std::mem::size_of::<MoveList>() <= 256 * 2 + 8);
}

#[test]
fn start_position_has_twenty_moves() {
    let m = moves("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    assert_eq!(m.len(), 20);
    assert!(m.contains(&"e2e4".to_string()) && m.contains(&"g1f3".to_string()));
}

#[test]
fn kiwipete_has_forty_eight_moves_including_both_castles() {
    let m = moves("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    assert_eq!(m.len(), 48, "{m:?}");
    assert!(m.contains(&"e1g1".to_string()) && m.contains(&"e1c1".to_string()));
    assert!(m.contains(&"d5e6".to_string()));
    assert!(m.contains(&"e2a6".to_string()));
}

#[test]
fn castling_edge_cases() {
    // Through check: the f1 square is attacked.
    assert!(!moves("4k3/8/8/8/8/8/5r2/4K2R w K - 0 1").contains(&"e1g1".to_string()));
    // Out of check.
    assert!(!moves("4k3/8/8/8/8/8/4r3/4K2R w K - 0 1").contains(&"e1g1".to_string()));
    // The b1 square may be attacked for queen-side castling.
    assert!(moves("4k3/8/8/8/8/8/1r6/R3K3 w Q - 0 1").contains(&"e1c1".to_string()));
    // But c1 and d1 may not be.
    assert!(!moves("4k3/8/8/8/8/8/3r4/R3K3 w Q - 0 1").contains(&"e1c1".to_string()));
    // Pieces in the way.
    assert!(!moves("4k3/8/8/8/8/8/8/RN2K3 w Q - 0 1").contains(&"e1c1".to_string()));
    // Black.
    assert!(moves("r3k2r/8/8/8/8/8/8/4K3 b kq - 0 1").contains(&"e8c8".to_string()));
}

#[test]
fn pins_and_checks() {
    // The knight on d2 is pinned by the rook on d8 and cannot move.
    let m = moves("3rk3/8/8/8/8/8/3N4/3K4 w - - 0 1");
    assert!(!m.iter().any(|s| s.starts_with("d2")), "{m:?}");
    // A pinned bishop may slide along the pin.
    let m = moves("4k3/8/8/8/7b/8/5B2/4K3 w - - 0 1");
    assert!(m.contains(&"f2g3".to_string()) && m.contains(&"f2h4".to_string()));
    assert!(!m.contains(&"f2e3".to_string()));
    // Double check: only king moves.
    let m = moves("4k3/8/8/8/8/3n4/8/4K2r w - - 0 1");
    assert!(m.iter().all(|s| s.starts_with("e1")), "{m:?}");
    assert_eq!(
        m,
        ["e1d2", "e1e2"],
        "d1 and f1 are x-rayed by the rook, f2 is covered by the knight"
    );
    // Single check with nothing to block or capture with: the king moves.
    let m = moves("4k3/8/8/8/8/8/8/r3K3 w - - 0 1");
    assert_eq!(m, ["e1d2", "e1e2", "e1f2"]);
    // The king may not step along the checking line (x-ray through the king).
    assert!(!m.contains(&"e1f1".to_string()));
}

#[test]
fn en_passant_including_the_discovered_check_along_the_rank() {
    let m = moves("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
    assert!(m.contains(&"e5d6".to_string()));
    // Capturing en passant would expose the king to the rook on the rank.
    let m = moves("8/8/8/KPp4r/8/8/8/4k3 w - c6 0 1");
    assert!(!m.contains(&"b5c6".to_string()), "{m:?}");
    // An en passant capture that removes the checking pawn is allowed.
    let m = moves("8/8/8/2k5/3Pp3/8/8/4K3 b - d3 0 1");
    assert!(m.contains(&"e4d3".to_string()), "{m:?}");
}

#[test]
fn promotions_and_capture_promotions() {
    let m = moves("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1");
    for s in ["a7a8q", "a7a8r", "a7a8b", "a7a8n", "a7b8q", "a7b8n"] {
        assert!(m.contains(&s.to_string()), "{m:?}");
    }
}

#[test]
fn uci_moves_parse_against_the_position() {
    let p = pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let m = p.parse_uci_move("e1g1").unwrap();
    assert!(m.is_castle());
    assert!(p.parse_uci_move("e1e2").is_none());
    assert!(p.parse_uci_move("zz").is_none());
    let p = pos("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1");
    assert_eq!(
        p.parse_uci_move("b7b8n").unwrap().promotion(),
        Some(rusty_chess_core::types::PieceKind::Knight)
    );
    assert!(p.parse_uci_move("b7b8").is_none());
}

#[test]
fn perft_matches_the_standard_positions_to_depth_three() {
    for (fen, counts) in [
        (
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            [20, 400, 8902],
        ),
        (
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            [48, 2039, 97862],
        ),
        ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", [14, 191, 2812]),
        (
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
            [6, 264, 9467],
        ),
        (
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
            [44, 1486, 62379],
        ),
        (
            "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
            [46, 2079, 89890],
        ),
    ] {
        let mut p = pos(fen);
        for (i, &expected) in counts.iter().enumerate() {
            assert_eq!(
                perft(&mut p, i as u32 + 1),
                expected,
                "{fen} depth {}",
                i + 1
            );
        }
        assert_eq!(p.to_fen(), fen, "perft must leave the position untouched");
    }
}

#[test]
fn perft_divide_sums_to_perft() {
    let mut p = pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let divide = perft_divide(&mut p, 2);
    assert_eq!(divide.len(), 48);
    assert_eq!(divide.iter().map(|(_, n)| n).sum::<u64>(), 2039);
    assert_eq!(perft(&mut p, 0), 1);
}
