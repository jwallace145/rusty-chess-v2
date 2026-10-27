//! The performance gate: measured telemetry against committed targets.

use rusty_chess_search::telemetry::{BenchRecord, PerftRecord, Record};
use rusty_chess_tools::gate::{Measured, Targets, evaluate};

const TARGETS: &str = r#"
# Minimum bars. Values are per environment because shared CI runners are slower
# and noisier than a quiet local machine. 0 means "not enforced yet".

[movegen]
min_perft_nps = { local = 100_000_000, ci = 40_000_000 }

[search]
min_nps = { local = 2_000_000, ci = 800_000 }

[efficiency]
min_fail_high_first = 0.90
max_ebf = 3.0
"#;

fn perft(nps: u64) -> Record {
    Record::Perft(PerftRecord {
        v: 1,
        positions: 60,
        max_depth: 6,
        nodes: nps * 2,
        elapsed_ms: 2000,
        nps,
        failures: 0,
    })
}

fn bench(nps: u64, fh_first: Option<f64>) -> Record {
    Record::Bench(BenchRecord {
        v: 1,
        positions: 50,
        depth: 12,
        hash_mb: 16,
        nodes: 1_000_000,
        nps,
        elapsed_ms: 500,
        stats: fh_first.map(|fh| rusty_chess_search::stats::StatsSummary {
            nodes: 1_000_000,
            qnodes: 300_000,
            qnode_share: 0.23,
            tt_probes: 100,
            tt_hits: 40,
            tt_cutoffs: 10,
            tt_collisions: 0,
            tt_hit: 0.4,
            fail_highs: 1000,
            fh_first: fh,
            cutoffs_by_stage: [900, 50, 30, 15, 5],
            null_move_cutoffs: 0,
            lmr_reductions: 0,
            lmr_researches: 0,
            futility_skips: 0,
        }),
    })
}

#[test]
fn targets_parse_per_environment() {
    let t = Targets::parse(TARGETS).unwrap();
    assert_eq!(t.movegen.min_perft_nps.get("local"), 100_000_000);
    assert_eq!(t.movegen.min_perft_nps.get("ci"), 40_000_000);
    assert_eq!(t.search.min_nps.get("ci"), 800_000);
    assert_eq!(t.efficiency.min_fail_high_first, 0.90);
    assert_eq!(t.efficiency.max_ebf, 3.0);
    assert!(Targets::parse("[movegen]\nmin_perft_nps = 'fast'\n").is_err());
}

#[test]
fn everything_above_the_bar_passes() {
    let t = Targets::parse(TARGETS).unwrap();
    let mut m = Measured::default();
    m.absorb(&[perft(50_000_000), bench(900_000, Some(0.95))]);
    m.ebf = Some(2.4);
    let outcome = evaluate(&t, &m, "ci", true);
    assert!(outcome.passed(), "{outcome}");
    assert_eq!(outcome.checks.len(), 4);
    assert!(outcome.checks.iter().all(|c| c.ok));
}

#[test]
fn speed_bars_are_noisy_and_only_enforced_when_asked() {
    let t = Targets::parse(TARGETS).unwrap();
    let mut m = Measured::default();
    m.absorb(&[perft(10_000_000), bench(100_000, Some(0.95))]);
    m.ebf = Some(2.0);
    let reported = evaluate(&t, &m, "ci", false);
    assert!(reported.passed(), "{reported}");
    assert_eq!(reported.checks.iter().filter(|c| !c.ok).count(), 2);
    assert!(reported.checks.iter().filter(|c| !c.ok).all(|c| c.noisy));
    let enforced = evaluate(&t, &m, "ci", true);
    assert!(!enforced.passed());
}

#[test]
fn efficiency_bars_are_deterministic_and_always_enforced() {
    let t = Targets::parse(TARGETS).unwrap();
    let mut m = Measured::default();
    m.absorb(&[perft(1_000_000_000), bench(9_000_000, Some(0.80))]);
    m.ebf = Some(3.5);
    let outcome = evaluate(&t, &m, "local", false);
    assert!(!outcome.passed());
    let failed: Vec<&str> = outcome
        .checks
        .iter()
        .filter(|c| !c.ok)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(
        failed,
        vec!["efficiency.min_fail_high_first", "efficiency.max_ebf"]
    );
}

#[test]
fn a_zero_target_is_not_enforced_and_a_missing_measurement_fails_a_nonzero_one() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 5, ci = 5 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n").unwrap();
    let m = Measured::default();
    let outcome = evaluate(&t, &m, "ci", true);
    assert!(!outcome.passed(), "{outcome}");
    let failed: Vec<&str> = outcome
        .checks
        .iter()
        .filter(|c| !c.ok)
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(failed, vec!["search.min_nps"]);
    assert!(
        outcome
            .checks
            .iter()
            .any(|c| c.name == "movegen.min_perft_nps" && c.ok && c.skipped)
    );
}

#[test]
fn targets_only_ratchet_up() {
    let old = Targets::parse(TARGETS).unwrap();
    let lowered = Targets::parse(&TARGETS.replace("ci = 40_000_000", "ci = 30_000_000")).unwrap();
    let raised = Targets::parse(&TARGETS.replace("ci = 40_000_000", "ci = 50_000_000")).unwrap();
    assert!(lowered.is_ratchet_from(&old).is_err());
    assert!(raised.is_ratchet_from(&old).is_ok());
    assert!(old.is_ratchet_from(&old).is_ok());
}
