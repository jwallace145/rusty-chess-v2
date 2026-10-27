//! The perft suite against the engine's move generator.
//!
//! `tests/perft.epd` is the coverage set; `rusty_chess_tools::perft` runs it
//! against `rusty_chess_core::movegen::perft`.

use rusty_chess_tools::perft::{Suite, run};

const EPD: &str = include_str!("perft.epd");

fn suite() -> Suite {
    Suite::parse(EPD).expect("tests/perft.epd parses")
}

#[test]
fn the_suite_parses_and_covers_the_standard_positions() {
    let suite = suite();
    assert!(
        suite.entries().len() >= 60,
        "{} positions",
        suite.entries().len()
    );
    assert!(suite.max_depth() >= 6);
    let fens: Vec<&str> = suite.entries().iter().map(|e| e.fen.as_str()).collect();
    for standard in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ] {
        assert!(fens.contains(&standard), "missing {standard}");
    }
    let start = &suite.entries()[0];
    assert_eq!(start.expected.last(), Some(&(6, 119_060_324)));
}

#[test]
fn no_position_appears_twice() {
    let suite = suite();
    let mut seen = std::collections::HashSet::new();
    for e in suite.entries() {
        assert!(seen.insert(e.fen.as_str()), "duplicate {}", e.fen);
    }
}

/// The real check. Depth is capped so the suite runs in seconds in a debug
/// build; `rusty-chess perft --suite tests/perft.epd` runs every depth in
/// release mode and is what CI measures.
#[test]
fn every_count_matches() {
    let max_depth: u32 = std::env::var("PERFT_MAX_DEPTH")
        .ok()
        .and_then(|d| d.parse().ok())
        .unwrap_or(4);
    let report = run(&suite(), max_depth, |fen, depth| {
        let mut pos =
            rusty_chess_core::position::Position::from_fen(fen).map_err(|e| e.to_string())?;
        Ok::<u64, String>(rusty_chess_core::movegen::perft(&mut pos, depth))
    });
    for failure in report.failures() {
        eprintln!(
            "{} D{} expected {} got {:?} {}",
            failure.fen,
            failure.depth,
            failure.expected,
            failure.got,
            failure.error.as_deref().unwrap_or("")
        );
    }
    assert!(
        report.passed(),
        "{} of {} checks failed",
        report.failures().len(),
        report.checks.len()
    );
}
