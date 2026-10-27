//! The openings generator: deterministic, balanced, distinct, parseable.

use rusty_chess_core::position::Position;
use rusty_chess_tools::openings::{Settings, generate};

#[test]
fn openings_are_deterministic_distinct_balanced_and_parse() {
    let settings = Settings {
        count: 12,
        plies: 6,
        depth: 2,
        max_cp: 60,
        seed: 42,
    };
    let a = generate(&settings);
    let b = generate(&settings);
    assert_eq!(a, b, "same settings, same openings");
    assert_eq!(a.len(), 12);
    let mut seen = std::collections::HashSet::new();
    for fen in &a {
        assert!(seen.insert(fen.clone()), "duplicate {fen}");
        let pos = Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
        assert!(!pos.in_check());
        assert_eq!(pos.fullmove_number(), 4, "{fen}: six plies from the start");
    }
    let other = generate(&Settings {
        seed: 43,
        ..settings
    });
    assert_ne!(a, other, "a different seed gives different openings");
}
