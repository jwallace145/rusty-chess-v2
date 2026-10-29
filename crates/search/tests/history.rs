//! Butterfly history with gravity, and quiet ordering by it.

use rusty_chess_core::r#move::Move;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::Color;
use rusty_chess_search::history::{Continuation, History, MAX_HISTORY, PieceTo};
use rusty_chess_search::movepick::{MovePicker, Stage};

fn mv(uci: &str, pos: &Position) -> Move {
    pos.parse_uci_move(uci).unwrap_or_else(|| panic!("{uci}"))
}

#[test]
fn bonuses_and_maluses_move_scores_in_opposite_directions() {
    let p = Position::startpos();
    let (a, b) = (mv("e2e4", &p), mv("d2d4", &p));
    let mut h = History::new();
    assert_eq!(h.get(Color::White, a), 0);
    h.update(Color::White, a, 4);
    h.update(Color::White, b, -4);
    assert!(h.get(Color::White, a) > 0);
    assert!(h.get(Color::White, b) < 0);
    assert_eq!(h.get(Color::Black, a), 0, "sides are separate");
}

#[test]
fn gravity_keeps_scores_bounded_and_saturates() {
    let p = Position::startpos();
    let a = mv("e2e4", &p);
    let mut h = History::new();
    let mut last = 0;
    for _ in 0..10_000 {
        h.update(Color::White, a, 30);
        let now = h.get(Color::White, a);
        assert!(now >= last, "monotone under repeated bonuses");
        assert!(now <= MAX_HISTORY);
        last = now;
    }
    assert!(last > MAX_HISTORY * 9 / 10, "approaches the bound: {last}");
    for _ in 0..10_000 {
        h.update(Color::White, a, -30);
    }
    assert!(h.get(Color::White, a) >= -MAX_HISTORY);
    assert!(h.get(Color::White, a) < -MAX_HISTORY * 9 / 10);
}

#[test]
fn the_bonus_grows_with_depth_but_is_capped() {
    assert!(History::bonus(4) > History::bonus(2));
    assert_eq!(
        History::bonus(40),
        History::bonus(60),
        "capped at deep depths"
    );
    assert!(History::bonus(40) < MAX_HISTORY);
}

#[test]
fn clearing_and_aging() {
    let p = Position::startpos();
    let a = mv("e2e4", &p);
    let mut h = History::new();
    h.update(Color::White, a, 10);
    let before = h.get(Color::White, a);
    h.age();
    let after = h.get(Color::White, a);
    assert!(after > 0 && after < before, "{before} -> {after}");
    h.clear();
    assert_eq!(h.get(Color::White, a), 0);
}

#[test]
fn the_picker_orders_quiets_by_history_after_killers() {
    let p = Position::startpos();
    let mut h = History::new();
    let best = mv("b1c3", &p);
    let second = mv("a2a3", &p);
    h.update(Color::White, best, 20);
    h.update(Color::White, second, 10);
    let killer = mv("g1f3", &p);
    let mut picker = MovePicker::new(Move::NULL).with_killers([killer, Move::NULL]);
    let mut out = Vec::new();
    while let Some(m) = picker.next_with(&p, &|m| h.get(p.side_to_move(), m)) {
        out.push((m, picker.stage()));
    }
    assert_eq!(out.len(), 20);
    assert_eq!(out[0], (killer, Stage::Killers));
    assert_eq!(out[1], (best, Stage::HistoryQuiets));
    assert_eq!(out[2], (second, Stage::HistoryQuiets));
}

#[test]
fn continuation_history_scores_a_move_given_what_came_before() {
    let p =
        Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1").unwrap();
    let e4 = PieceTo { piece: 0, to: 28 }; // white pawn to e4 (a white pawn is piece 0)
    let nf6 = PieceTo::of(&p, p.parse_uci_move("g8f6").unwrap());
    let nc6 = PieceTo::of(&p, p.parse_uci_move("b8c6").unwrap());
    assert_eq!(nf6.piece, 6 + 1, "a black knight");
    let mut c = Continuation::new();
    assert_eq!(c.get(e4, nf6), 0);
    c.update(e4, nf6, 400);
    c.update(e4, nc6, -400);
    assert!(c.get(e4, nf6) > 0 && c.get(e4, nc6) < 0);
    // Keyed by the earlier move: after a different move the scores are untouched.
    let d4 = PieceTo { piece: 0, to: 27 };
    assert_eq!(c.get(d4, nf6), 0);
    // No previous move: nothing scored, nothing stored.
    c.update(PieceTo::NONE, nf6, 1000);
    assert_eq!(c.get(PieceTo::NONE, nf6), 0);
    // Gravity keeps the score bounded; aging halves it; clear zeroes it.
    for _ in 0..200 {
        c.update(e4, nf6, 1200);
    }
    assert!(c.get(e4, nf6) <= MAX_HISTORY);
    let before = c.get(e4, nf6);
    c.age();
    assert_eq!(c.get(e4, nf6), before / 2);
    c.clear();
    assert_eq!(c.get(e4, nf6), 0);
}
