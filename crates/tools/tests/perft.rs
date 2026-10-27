//! The perft suite: EPD parsing and the runner, exercised with a fake perft.

use rusty_chess_tools::perft::{Suite, run};

const EPD: &str = "\
# comment lines and blank lines are skipped

rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 ;D1 20 ;D2 400 ;D3 8902
8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 1
";

#[test]
fn parses_fen_and_depth_counts() {
    let suite = Suite::parse(EPD).unwrap();
    assert_eq!(suite.entries().len(), 2);
    let first = &suite.entries()[0];
    assert_eq!(
        first.fen,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
    assert_eq!(first.expected, vec![(1, 20), (2, 400), (3, 8902)]);
    assert_eq!(suite.entries()[1].expected, vec![(1, 1)]);
    assert_eq!(suite.max_depth(), 3);
}

#[test]
fn rejects_malformed_lines_with_the_line_number() {
    let err = Suite::parse("8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 one\n").unwrap_err();
    assert!(err.to_string().contains("line 1"), "{err}");
    let err = Suite::parse("8/8/8/8/8/K7/P7/k7 w - - 0 1 ;X1 1\n").unwrap_err();
    assert!(err.to_string().contains("line 1"), "{err}");
    let err = Suite::parse("8/8/8/8/8/K7/P7/k7 w - - 0 1\n").unwrap_err();
    assert!(err.to_string().contains("no depth"), "{err}");
    let err = Suite::parse("not a fen ;D1 1\n").unwrap_err();
    assert!(err.to_string().contains("fen"), "{err}");
}

#[test]
fn runner_reports_mismatches_and_totals() {
    let suite = Suite::parse(EPD).unwrap();
    // A fake perft that is right about everything except D3 of the start position.
    let report = run(&suite, 3, |fen, depth| {
        Ok::<u64, String>(match (fen.starts_with("rnbqkbnr"), depth) {
            (true, 1) => 20,
            (true, 2) => 400,
            (true, 3) => 8903,
            (false, 1) => 1,
            _ => unreachable!(),
        })
    });
    assert_eq!(report.checks.len(), 4);
    let failures = report.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].depth, 3);
    assert_eq!((failures[0].expected, failures[0].got), (8902, Some(8903)));
    assert_eq!(report.nodes, 20 + 400 + 8903 + 1);
    assert!(!report.passed());
}

#[test]
fn runner_respects_max_depth_and_surfaces_errors() {
    let suite = Suite::parse(EPD).unwrap();
    let report = run(&suite, 2, |fen, depth| {
        if fen.starts_with("8/8") {
            Err("cannot parse".to_string())
        } else {
            Ok(if depth == 1 { 20 } else { 400 })
        }
    });
    assert_eq!(report.checks.len(), 3);
    assert!(report.checks.iter().all(|c| c.depth <= 2));
    let failures = report.failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].got, None);
    assert_eq!(failures[0].error.as_deref(), Some("cannot parse"));
    assert!(!report.passed());

    let record = report.record(2);
    assert_eq!(record.positions, 2);
    assert_eq!(record.max_depth, 2);
    assert_eq!(record.failures, 1);
    assert_eq!(record.nodes, 420);
}

#[test]
fn a_fully_correct_run_passes() {
    let suite = Suite::parse("8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 1 ;D2 3\n").unwrap();
    let report = run(&suite, 9, |_, depth| {
        Ok::<u64, String>(if depth == 1 { 1 } else { 3 })
    });
    assert!(report.passed());
    assert_eq!(report.failures().len(), 0);
    assert_eq!(report.record(9).failures, 0);
}
