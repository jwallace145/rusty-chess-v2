//! The staged move picker: TT move, good captures, killers, history quiets,
//! bad captures; every legal move exactly once.

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_search::movepick::{MovePicker, Stage};
use rusty_chess_search::see::see;

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"))
}

fn drain(p: &Position, tt: Move) -> Vec<(Move, Stage)> {
    let mut picker = MovePicker::new(tt);
    let mut out = Vec::new();
    while let Some(m) = picker.next(p) {
        out.push((m, picker.stage()));
    }
    out
}

#[test]
fn every_legal_move_exactly_once_with_the_tt_move_first() {
    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "8/8/8/2k5/3Pp3/8/8/4K3 b - d3 0 1",
        "4k3/8/8/8/8/3n4/8/4K2r w - - 0 1",
    ] {
        let p = pos(fen);
        let mut legal = MoveList::new();
        p.legal_moves(&mut legal);
        let tt = legal.as_slice()[legal.len() / 2];
        let picked = drain(&p, tt);
        assert_eq!(picked.len(), legal.len(), "{fen}");
        assert_eq!(picked[0], (tt, Stage::TtMove), "{fen}");
        let mut seen: Vec<String> = picked.iter().map(|(m, _)| m.to_string()).collect();
        seen.sort();
        seen.dedup();
        assert_eq!(seen.len(), legal.len(), "{fen}: a move was yielded twice");
        // Stages come out in order.
        let stages: Vec<Stage> = picked.iter().map(|(_, s)| *s).collect();
        assert!(stages.windows(2).all(|w| w[0] <= w[1]), "{fen}: {stages:?}");
    }
}

#[test]
fn a_tt_move_that_is_not_legal_is_skipped() {
    let p = pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let bogus = Move::new(
        "a1".parse().unwrap(),
        "a8".parse().unwrap(),
        rusty_chess_core::r#move::MoveFlag::Capture,
    );
    let picked = drain(&p, bogus);
    let mut legal = MoveList::new();
    p.legal_moves(&mut legal);
    assert_eq!(picked.len(), legal.len());
    assert!(picked.iter().all(|(_, s)| *s != Stage::TtMove));
    assert_eq!(drain(&p, Move::NULL).len(), legal.len());
}

#[test]
fn good_captures_come_before_quiets_and_bad_captures_after() {
    // Kiwipete: e2a6 (bishop takes bishop, defended by the knight b6 → SEE 0 or so, good),
    // d5e6 (pawn takes pawn, good), f3f6 (queen takes knight defended by the pawn g7 → bad), f3h3 (queen takes pawn defended by ... ) etc.
    let p = pos("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let picked = drain(&p, Move::NULL);
    let stage_of = |uci: &str| {
        picked
            .iter()
            .find(|(m, _)| m.to_string() == uci)
            .map(|(_, s)| *s)
            .unwrap_or_else(|| panic!("{uci} missing"))
    };
    assert_eq!(stage_of("d5e6"), Stage::GoodCaptures);
    assert_eq!(
        stage_of("f3f6"),
        Stage::BadCaptures,
        "queen takes a knight defended by a pawn"
    );
    assert_eq!(stage_of("e1g1"), Stage::HistoryQuiets);
    // Every good capture has non-negative SEE and every bad one negative.
    for (m, s) in &picked {
        match s {
            Stage::GoodCaptures => assert!(see(&p, *m) >= 0, "{m} is good but SEE {}", see(&p, *m)),
            Stage::BadCaptures => assert!(see(&p, *m) < 0, "{m} is bad but SEE {}", see(&p, *m)),
            _ => {}
        }
    }
    // Within good captures, the most valuable victim comes first.
    let good: Vec<&Move> = picked
        .iter()
        .filter(|(_, s)| *s == Stage::GoodCaptures)
        .map(|(m, _)| m)
        .collect();
    let victim = |m: &Move| {
        p.piece_at(m.to())
            .map_or(0, |pc| [100, 320, 330, 500, 900, 0][pc.kind.index()])
    };
    assert!(
        good.windows(2).all(|w| victim(w[0]) >= victim(w[1])),
        "{good:?}"
    );
}

#[test]
fn killer_and_history_stages_exist_but_are_empty_until_their_prs() {
    let p = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let picked = drain(&p, Move::NULL);
    assert!(picked.iter().all(|(_, s)| *s != Stage::Killers));
    assert_eq!(
        picked
            .iter()
            .filter(|(_, s)| *s == Stage::HistoryQuiets)
            .count(),
        20
    );
    assert_eq!(Stage::TtMove as usize, 0);
    assert_eq!(Stage::BadCaptures as usize, 4);
}
