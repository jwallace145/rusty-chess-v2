//! Killer moves: quiet moves that caused a beta cutoff at a ply, tried early
//! at that ply in sibling nodes.

use rusty_chess_core::r#move::{Move, MoveFlag};
use rusty_chess_core::position::Position;
use rusty_chess_search::killers::Killers;
use rusty_chess_search::movepick::{MovePicker, Stage};

fn mv(uci: &str, pos: &Position) -> Move {
    pos.parse_uci_move(uci).unwrap_or_else(|| panic!("{uci}"))
}

#[test]
fn two_slots_per_ply_newest_first_without_duplicates() {
    let p = Position::startpos();
    let (a, b, c) = (mv("e2e4", &p), mv("d2d4", &p), mv("g1f3", &p));
    let mut k = Killers::new();
    assert_eq!(k.get(3), [Move::NULL; 2]);
    k.store(3, a);
    assert_eq!(k.get(3), [a, Move::NULL]);
    k.store(3, b);
    assert_eq!(k.get(3), [b, a]);
    k.store(3, b);
    assert_eq!(
        k.get(3),
        [b, a],
        "storing the first killer again changes nothing"
    );
    k.store(3, c);
    assert_eq!(k.get(3), [c, b], "the oldest falls out");
    assert_eq!(k.get(4), [Move::NULL; 2], "plies are independent");
    k.clear();
    assert_eq!(k.get(3), [Move::NULL; 2]);
}

#[test]
fn the_picker_yields_legal_killers_in_their_stage_and_never_twice() {
    let p = Position::startpos();
    let killers = [mv("g1f3", &p), mv("e2e4", &p)];
    let mut picker = MovePicker::new(Move::NULL).with_killers(killers);
    let mut seen = Vec::new();
    while let Some(m) = picker.next(&p) {
        seen.push((m, picker.stage()));
    }
    assert_eq!(seen.len(), 20);
    assert_eq!(seen[0], (killers[0], Stage::Killers));
    assert_eq!(seen[1], (killers[1], Stage::Killers));
    assert_eq!(seen.iter().filter(|(m, _)| *m == killers[0]).count(), 1);
    // A killer that is not legal here (from a sibling with a different position) is skipped.
    let bogus = Move::new(
        "e4".parse().unwrap(),
        "e5".parse().unwrap(),
        MoveFlag::Quiet,
    );
    let mut picker = MovePicker::new(Move::NULL).with_killers([bogus, Move::NULL]);
    let mut n = 0;
    while let Some(m) = picker.next(&p) {
        assert_ne!(m, bogus);
        n += 1;
    }
    assert_eq!(n, 20);
}
