//! The PR bench report: a compact Markdown comparison of two runs.

use rusty_chess_search::stats::StatsSummary;
use rusty_chess_search::telemetry::{BenchRecord, PerftRecord, Record};
use rusty_chess_tools::gate::{Measured, Targets};
use rusty_chess_tools::report::{Meta, render};

fn stats(fh_first: f64) -> StatsSummary {
    StatsSummary {
        nodes: 1,
        qnodes: 0,
        qnode_share: 0.0,
        tt_probes: 0,
        tt_hits: 0,
        tt_cutoffs: 0,
        tt_collisions: 0,
        tt_hit: 0.0,
        fail_highs: 1,
        fh_first,
        cutoffs_by_stage: [0; 5],
        rfp_cutoffs: 0,
        null_move_cutoffs: 0,
        lmr_reductions: 0,
        lmr_researches: 0,
        futility_skips: 0,
        lmp_skips: 0,
        mdp_cutoffs: 0,
        qsee_skips: 0,
        lmp_checks_kept: 0,
        aspiration_researches: 0,
    }
}

fn run(signature: u64, nps: u64, fh_first: f64, ebf: f64, perft_nps: Option<u64>) -> Measured {
    let mut m = Measured::default();
    let mut records = vec![Record::Bench(BenchRecord {
        v: 1,
        positions: 50,
        depth: 12,
        hash_mb: 16,
        cpu: None,
        nodes: signature,
        nps,
        elapsed_ms: 1234,
        stats: Some(stats(fh_first)),
    })];
    if let Some(p) = perft_nps {
        records.push(Record::Perft(PerftRecord {
            v: 1,
            positions: 64,
            max_depth: 5,
            nodes: p * 3,
            elapsed_ms: 3000,
            nps: p,
            failures: 0,
        }));
    }
    m.absorb(&records);
    m.ebf = Some(ebf);
    m
}

fn targets() -> Targets {
    Targets::parse(
        "[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[comment]\nspeed_noise_pct = 10.0\nefficiency_noise_pct = 1.0\n",
    )
    .unwrap()
}

fn meta() -> Meta {
    Meta {
        base_label: Some("v0.1.0".into()),
        head_label: "ab807a0".into(),
        gate_passed: Some(true),
        gate_table: Some("| Bar | Measured |\n| --- | ---: |\n".into()),
        noisy_enforced: false,
    }
}

#[test]
fn measured_carries_the_signature_and_run_configuration() {
    let m = run(412_331, 2_310_000, 0.91, 2.1, Some(118_000_000));
    assert_eq!(m.signature, Some(412_331));
    assert_eq!(m.bench_config, Some((50, 12, 16)));
    assert_eq!(m.perft_failures, Some(0));
    assert_eq!(m.perft_positions, Some(64));
}

#[test]
fn deltas_improvements_and_regressions_beyond_the_noise_threshold() {
    let base = run(412_331, 2_000_000, 0.90, 2.20, Some(100_000_000));
    let head = run(398_120, 2_300_000, 0.92, 2.05, Some(90_000_000));
    let md = render(Some(&base), &head, &targets(), &meta());
    println!("{md}");
    let row = |name: &str| {
        md.lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap_or_else(|| panic!("no row {name}\n{md}"))
            .to_string()
    };
    assert!(
        row("Bench signature").contains("412,331") && row("Bench signature").contains("398,120")
    );
    assert!(row("Bench signature").contains("changed"));
    assert!(row("Search NPS").contains("2.00M") && row("Search NPS").contains("2.30M"));
    assert!(
        row("Search NPS").contains("+15.0%") && row("Search NPS").ends_with("| ⚪ |"),
        "{}",
        row("Search NPS")
    );
    assert!(
        row("Perft NPS").contains("−10.0%") && row("Perft NPS").ends_with("| ⚪ |"),
        "{}",
        row("Perft NPS")
    );
    assert!(
        row("Fail-high first").contains("90.0%")
            && row("Fail-high first").contains("+2.0 pt")
            && row("Fail-high first").ends_with("| 🟢 |")
    );
    assert!(
        row("EBF").contains("2.20")
            && row("EBF").contains("2.05")
            && row("EBF").ends_with("| 🟢 |"),
        "{}",
        row("EBF")
    );
}

#[test]
fn changes_within_the_noise_threshold_are_neutral() {
    let base = run(1, 2_000_000, 0.900, 2.20, None);
    let head = run(1, 2_100_000, 0.905, 2.21, None);
    let md = render(Some(&base), &head, &targets(), &meta());
    let row = |name: &str| {
        md.lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap()
            .to_string()
    };
    assert!(row("Bench signature").contains("same") && row("Bench signature").ends_with("| ⚪ |"));
    assert!(
        row("Search NPS").contains("+5.0%") && row("Search NPS").ends_with("| ⚪ |"),
        "{}",
        row("Search NPS")
    );
    assert!(row("Fail-high first").ends_with("| ⚪ |"));
    assert!(row("EBF").ends_with("| ⚪ |"));
    // Perft was not measured on either side: no row at all.
    assert!(!md.contains("| Perft"), "{md}");
}

#[test]
fn a_missing_baseline_shows_the_head_values_only() {
    let head = run(7, 2_000_000, 0.9, 2.0, Some(50_000_000));
    let mut m = meta();
    m.base_label = None;
    let md = render(None, &head, &targets(), &m);
    assert!(md.contains("no baseline"), "{md}");
    let row = md
        .lines()
        .find(|l| l.starts_with("| Search NPS |"))
        .unwrap();
    assert!(row.contains("2.00M") && row.ends_with("| 🆕 |"), "{row}");
}

#[test]
fn the_comment_is_compact_and_carries_the_gate_in_a_collapsed_section() {
    let base = run(1, 2_000_000, 0.9, 2.2, Some(100_000_000));
    let head = run(1, 2_000_000, 0.9, 2.2, Some(100_000_000));
    let md = render(Some(&base), &head, &targets(), &meta());
    assert!(md.starts_with("### "), "{md}");
    assert!(
        md.lines().count() < 45,
        "{} lines\n{md}",
        md.lines().count()
    );
    // Everything after the gate line is folded away: the visible part stays short.
    let visible = md.split("<details>").next().unwrap();
    assert!(
        visible.lines().count() <= 14,
        "{} visible lines\n{visible}",
        visible.lines().count()
    );
    assert!(md.contains("<details>") && md.contains("</details>"));
    assert!(md.contains("| Bar | Measured |"));
    assert!(md.contains("v0.1.0") && md.contains("ab807a0"));
    assert!(md.contains("Performance gate: **pass**"));
    assert!(md.contains("±10%") && md.contains("±1.0 pt"));
    let mut failed = meta();
    failed.gate_passed = Some(false);
    assert!(render(Some(&base), &head, &targets(), &failed).contains("Performance gate: **fail**"));
}

#[test]
fn perft_failures_are_called_out() {
    let mut head = run(1, 2_000_000, 0.9, 2.2, Some(100_000_000));
    head.perft_failures = Some(3);
    let md = render(None, &head, &targets(), &meta());
    let row = md
        .lines()
        .find(|l| l.starts_with("| Perft suite |"))
        .unwrap();
    assert!(
        row.contains("3 failures") && row.ends_with("| 🔴 |"),
        "{row}"
    );
}

#[test]
fn a_collapsed_section_explains_each_row_that_is_shown() {
    let base = run(1, 2_000_000, 0.9, 2.2, Some(100_000_000));
    let head = run(1, 2_000_000, 0.9, 2.2, Some(100_000_000));
    let md = render(Some(&base), &head, &targets(), &meta());
    // Two collapsed sections: details, then the explainer; neither is open by default.
    assert_eq!(md.matches("<details>").count(), 2, "{md}");
    assert!(!md.contains("<details open"), "{md}");
    let explainer = md
        .split("<summary>What these numbers mean</summary>")
        .nth(1)
        .unwrap_or_else(|| panic!("no explainer\n{md}"));
    for row in [
        "Bench signature",
        "Search NPS",
        "Perft NPS",
        "Perft suite",
        "Fail-high first",
        "EBF",
    ] {
        assert!(
            explainer.contains(&format!("**{row}**")),
            "explainer lacks {row}\n{explainer}"
        );
    }
    assert!(
        explainer.contains("🟢")
            && explainer.contains("🔴")
            && explainer.contains("⚪")
            && explainer.contains("🆕")
    );
    assert!(explainer.contains("perf-targets.toml"));
    assert!(
        md.lines().count() < 50,
        "{} lines\n{md}",
        md.lines().count()
    );

    // Rows that are not shown are not explained.
    let head = run(1, 2_000_000, 0.9, 2.2, None);
    let md = render(None, &head, &targets(), &meta());
    assert!(!md.contains("**Perft NPS**"), "{md}");
}

#[test]
fn a_zero_baseline_reads_as_new_not_as_a_zero_percent_change() {
    let base = run(0, 0, 0.0, 0.0, None);
    let head = run(68_269_801, 7_900_000, 0.7, 0.0, None);
    let md = render(Some(&base), &head, &targets(), &meta());
    let row = md
        .lines()
        .find(|l| l.starts_with("| Search NPS |"))
        .unwrap();
    assert!(
        row.contains("| 0 | 7.90M |") && row.ends_with("| 🆕 |"),
        "{row}"
    );
    let row = md
        .lines()
        .find(|l| l.starts_with("| Fail-high first |"))
        .unwrap();
    assert!(row.ends_with("| 🆕 |"), "{row}");
    let row = md
        .lines()
        .find(|l| l.starts_with("| Bench signature |"))
        .unwrap();
    assert!(row.contains("changed"), "{row}");
}

#[test]
fn speed_is_judged_by_the_paired_row_and_absolute_nps_is_informational() {
    let base = run(1, 2_000_000, 0.9, 2.2, Some(300_000_000));
    // A slower runner: absolute NPS 30% lower, but paired speed +4% with a tight interval.
    let mut head = run(1, 1_400_000, 0.9, 2.2, Some(210_000_000));
    head.speed = Some((1.04, 1.03, 1.05));
    head.instructions_per_node = Some(980.0);
    let mut base = base;
    base.instructions_per_node = Some(1000.0);
    let md = render(Some(&base), &head, &targets(), &meta());
    let row = |name: &str| {
        md.lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap_or_else(|| panic!("no row {name}\n{md}"))
            .to_string()
    };
    assert!(
        row("Search NPS").contains("−30.0%") && row("Search NPS").ends_with("| ⚪ |"),
        "{}",
        row("Search NPS")
    );
    assert!(row("Perft NPS").ends_with("| ⚪ |"), "{}", row("Perft NPS"));
    assert!(
        row("Speed vs base").contains("| same runner | +4.0% | ±1.0% | 🟢 |"),
        "{}",
        row("Speed vs base")
    );
    assert!(
        row("Instructions / node").contains("| 1000 | 980 | −2.0% | 🟢 |"),
        "{}",
        row("Instructions / node")
    );
    // An interval that straddles zero is not a speed change.
    head.speed = Some((1.01, 0.99, 1.03));
    let md = render(Some(&base), &head, &targets(), &meta());
    assert!(
        md.lines()
            .any(|l| l.starts_with("| Speed vs base |") && l.ends_with("| ⚪ |")),
        "{md}"
    );
    head.speed = Some((0.95, 0.94, 0.96));
    let md = render(Some(&base), &head, &targets(), &meta());
    assert!(
        md.lines()
            .any(|l| l.starts_with("| Speed vs base |") && l.ends_with("| 🔴 |")),
        "{md}"
    );
    assert!(md.contains("**Speed vs base**") && md.contains("**Instructions / node**"));
}

#[test]
fn a_changed_signature_hands_the_speed_verdict_to_time_to_depth() {
    // Fewer, dearer nodes: NPS down 35%, but the same depth in 16% less time.
    let base = run(3_629_136, 2_000_000, 0.9, 1.8, None);
    let mut head = run(2_566_403, 1_300_000, 0.9, 2.0, None);
    head.speed = Some((0.65, 0.62, 0.68));
    head.time_to_depth = Some((0.84, 0.81, 0.87));
    let md = render(Some(&base), &head, &targets(), &meta());
    let row = |name: &str| {
        md.lines()
            .find(|l| l.starts_with(&format!("| {name} |")))
            .unwrap_or_else(|| panic!("no row {name}\n{md}"))
            .to_string()
    };
    assert!(
        row("Speed vs base").contains("per node") && row("Speed vs base").ends_with("| ⚪ |"),
        "{}",
        row("Speed vs base")
    );
    assert!(
        row("Time to depth").contains("−16.0%") && row("Time to depth").ends_with("| 🟢 |"),
        "{}",
        row("Time to depth")
    );
    assert!(md.contains("**Time to depth**"), "{md}");
    // Same signature: NPS keeps its verdict, and more time is a regression.
    let mut head = run(3_629_136, 1_300_000, 0.9, 1.8, None);
    head.speed = Some((0.95, 0.94, 0.96));
    head.time_to_depth = Some((1.05, 1.04, 1.06));
    let md = render(Some(&base), &head, &targets(), &meta());
    assert!(
        md.lines()
            .any(|l| l.starts_with("| Speed vs base |") && l.ends_with("| 🔴 |")),
        "{md}"
    );
    assert!(
        md.lines()
            .any(|l| l.starts_with("| Time to depth |") && l.ends_with("| 🔴 |")),
        "{md}"
    );
}
