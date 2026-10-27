//! Static exchange evaluation: the material outcome of a capture sequence on
//! one square when both sides capture only while it pays.

use rusty_chess_core::position::Position;
use rusty_chess_search::see::{see, see_ge};

fn eval(fen: &str, uci: &str) -> i32 {
    let p = Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
    let m = p
        .parse_uci_move(uci)
        .unwrap_or_else(|| panic!("{uci} is not legal in {fen}"));
    see(&p, m)
}

#[test]
fn an_undefended_pawn_is_worth_a_pawn() {
    assert_eq!(eval("4k3/8/8/4p3/8/8/4R3/4K3 w - - 0 1", "e2e5"), 100);
}

#[test]
fn a_defended_pawn_taken_by_a_rook_loses_the_exchange() {
    // Rxe5 dxe5: +100 -500.
    assert_eq!(eval("4k3/8/3p4/4p3/8/8/4R3/4K3 w - - 0 1", "e2e5"), -400);
}

#[test]
fn x_ray_attackers_join_the_sequence() {
    // Rooks on e2 and e1 against a pawn on e5 defended by d6: Rxe5 dxe5 Rxe5 = +100 -500 +100.
    assert_eq!(eval("4k3/8/3p4/4p3/8/8/4R3/4R1K1 w - - 0 1", "e2e5"), -300);
    // Add a second defender (a knight on c6) and black wins the second rook too; black stops when it stops paying.
    // Rxe5 dxe5 Rxe5 Nxe5: +100 -500 +100 -500 = -800, but white would not play the second capture,
    // so from white's view the sequence is Rxe5 dxe5 = -400.
    assert_eq!(eval("4k3/8/2np4/4p3/8/8/4R3/4R1K1 w - - 0 1", "e2e5"), -400);
}

#[test]
fn the_defender_declines_when_recapturing_loses() {
    // Bishop takes a knight defended by a pawn: +320 -330 would lose for white? No: white gains 320, loses 330 → -10.
    assert_eq!(
        eval("4k3/8/3p4/4n3/8/8/1B6/4K3 w - - 0 1", "b2e5"),
        320 - 330
    );
    // Queen takes a pawn defended by a pawn: +100 -900.
    assert_eq!(
        eval("4k3/8/3p4/4p3/8/8/4Q3/4K3 w - - 0 1", "e2e5"),
        100 - 900
    );
}

#[test]
fn en_passant_and_promotions() {
    // En passant captures a pawn: +100, the capturing pawn then recaptured by nothing.
    assert_eq!(eval("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", "e5d6"), 100);
    // A capture-promotion gains the captured piece plus the promotion gain (queen − pawn).
    assert_eq!(
        eval("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1", "a7b8q"),
        320 + 900 - 100
    );
    // A quiet promotion gains the promotion.
    assert_eq!(eval("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", "a7a8q"), 900 - 100);
}

#[test]
fn a_king_capture_counts_the_captured_piece() {
    // Kxd2 of an undefended pawn: +100. The move generator already guarantees it is not into check.
    assert_eq!(eval("4k3/8/8/8/8/8/3pK3/8 w - - 0 1", "e2d2"), 100);
    // A king may start a sequence but can never be captured in it: Kxd2 with the pawn defended
    // is illegal, so the generator never asks. Rxd7 defended by the king: +100 then Kxd7 -500.
    assert_eq!(eval("4k3/3p4/8/8/8/8/8/3RK3 w - - 0 1", "d1d7"), 100 - 500);
}

#[test]
fn see_ge_is_a_threshold_test() {
    let fen = "4k3/8/3p4/4p3/8/8/4R3/4K3 w - - 0 1";
    let p = Position::from_fen(fen).unwrap();
    let m = p.parse_uci_move("e2e5").unwrap();
    assert!(see_ge(&p, m, -400));
    assert!(!see_ge(&p, m, -399));
    assert!(!see_ge(&p, m, 0));
    let fen = "4k3/8/8/4p3/8/8/4R3/4K3 w - - 0 1";
    let p = Position::from_fen(fen).unwrap();
    let m = p.parse_uci_move("e2e5").unwrap();
    assert!(see_ge(&p, m, 0) && see_ge(&p, m, 100) && !see_ge(&p, m, 101));
}

#[test]
fn quiet_moves_evaluate_to_zero() {
    assert_eq!(
        eval(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "e2e4"
        ),
        0
    );
}
