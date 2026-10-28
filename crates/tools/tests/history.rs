//! Release benchmark history: the table, the changelog section, the notes.

use rusty_chess_search::stats::StatsSummary;
use rusty_chess_search::telemetry::{BenchRecord, PerftRecord, Record};
use rusty_chess_tools::gate::Measured;
use rusty_chess_tools::history::{
    changelog_section, empty_document, insert_into_changelog, release_notes, row, upsert_row,
};

fn measured(signature: u64, nps: u64, fh: f64, ebf: f64) -> Measured {
    let mut m = Measured::default();
    m.absorb(&[
        Record::Bench(BenchRecord {
            v: 1,
            positions: 50,
            depth: 6,
            hash_mb: 16,
            cpu: None,
            nodes: signature,
            nps,
            elapsed_ms: 1000,
            stats: Some(StatsSummary {
                nodes: 1,
                qnodes: 0,
                qnode_share: 0.0,
                tt_probes: 10,
                tt_hits: 3,
                tt_cutoffs: 0,
                tt_collisions: 0,
                tt_hit: 0.3,
                fail_highs: 1,
                fh_first: fh,
                cutoffs_by_stage: [0; 5],
                rfp_cutoffs: 0,
                null_move_cutoffs: 0,
                lmr_reductions: 0,
                lmr_researches: 0,
                futility_skips: 0,
                aspiration_researches: 0,
            }),
        }),
        Record::Perft(PerftRecord {
            v: 1,
            positions: 63,
            max_depth: 7,
            nodes: 1,
            elapsed_ms: 1,
            nps: 250_000_000,
            failures: 0,
        }),
    ]);
    m.ebf = Some(ebf);
    m
}

#[test]
fn rows_are_inserted_newest_first_and_only_once() {
    let doc = empty_document();
    let a = row(
        "0.4.0",
        "2026-10-27",
        &measured(14_255_682, 6_850_000, 0.787, 4.62),
    );
    assert_eq!(
        a,
        "| v0.4.0 | 2026-10-27 | 14,255,682 | 6.85M | 250.00M | pass (63) | 78.7% | 4.62 | 30.0% | — |"
    );
    let doc = upsert_row(&doc, "0.4.0", &a).unwrap();
    let b = row(
        "0.5.0",
        "2026-10-28",
        &measured(14_942_735, 11_100_000, 0.856, 4.74),
    );
    let doc = upsert_row(&doc, "0.5.0", &b).unwrap();
    let rows: Vec<&str> = doc.lines().filter(|l| l.starts_with("| v")).collect();
    assert_eq!(rows, vec![b.as_str(), a.as_str()]);
    assert!(
        upsert_row(&doc, "0.5.0", &b).is_none(),
        "recording is idempotent"
    );
    assert!(upsert_row("no table here\n", "0.6.0", &b).is_none());
}

const CHANGELOG: &str = "# Changelog\n\n## [0.5.0](https://x/compare/v0.4.0...v0.5.0) (2026-10-28)\n\n\n### Features\n\n* **search:** add a staged move picker ([#29](x))\n\n## [0.4.0](https://x) (2026-10-27)\n\n\n### Features\n\n* **search:** add a transposition table\n";

#[test]
fn the_changelog_section_goes_under_its_own_version_once() {
    let now = measured(14_942_735, 11_100_000, 0.856, 4.74);
    let prev = measured(14_255_682, 6_850_000, 0.787, 4.62);
    let section = changelog_section(&now, Some(("0.4.0", &prev)));
    assert!(section.starts_with("### Benchmarks\n"), "{section}");
    assert!(
        section.contains("fail-high first +6.9 pt")
            && section.contains("EBF +0.12")
            && section.contains("search NPS +62.0%"),
        "{section}"
    );
    let out = insert_into_changelog(CHANGELOG, "0.5.0", &section).unwrap();
    let v5 = &out[out.find("## [0.5.0]").unwrap()..out.find("## [0.4.0]").unwrap()];
    assert!(
        v5.contains("### Features") && v5.contains("### Benchmarks"),
        "{v5}"
    );
    assert!(v5.find("### Features") < v5.find("### Benchmarks"));
    assert!(!out[out.find("## [0.4.0]").unwrap()..].contains("### Benchmarks"));
    assert!(
        insert_into_changelog(&out, "0.5.0", &section).is_none(),
        "only once"
    );
    assert!(insert_into_changelog(CHANGELOG, "9.9.9", &section).is_none());
}

#[test]
fn release_notes_show_deltas_against_the_previous_release() {
    let now = measured(14_942_735, 11_100_000, 0.856, 4.74);
    let prev = measured(14_255_682, 6_850_000, 0.787, 4.62);
    let notes = release_notes("0.5.0", &now, Some(("0.4.0", &prev)));
    assert!(notes.starts_with("## Benchmarks\n"), "{notes}");
    assert!(notes.contains("| | v0.4.0 | v0.5.0 | Δ |"), "{notes}");
    assert!(
        notes.contains("| Bench signature | 14,255,682 | 14,942,735 | changed |"),
        "{notes}"
    );
    assert!(
        notes.contains("| Search NPS | 6.85M | 11.10M | +62.0% |"),
        "{notes}"
    );
    assert!(
        notes.contains("| Fail-high first | 78.7% | 85.6% | +6.9 pt |"),
        "{notes}"
    );
    assert!(notes.contains("| EBF | 4.62 | 4.74 | +0.12 |"), "{notes}");
    assert!(notes.contains("Perft suite: pass on 63 positions."));
    let first = release_notes("0.1.0", &now, None);
    assert!(
        first.contains("| | previous | v0.1.0 | Δ |")
            && first.contains("| Bench signature | — | 14,942,735 |  |"),
        "{first}"
    );
}

#[test]
fn the_summary_is_machine_readable_json() {
    let m = measured(14_942_735, 11_100_000, 0.856, 4.74);
    let text = rusty_chess_tools::history::summary_json("0.5.0", "2026-10-28", &m);
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], "v0.5.0");
    assert_eq!(v["bench"]["signature"], 14_942_735);
    assert_eq!(v["bench"]["depth"], 6);
    assert_eq!(v["efficiency"]["ebf"], 4.74);
    assert_eq!(v["perft"]["failures"], 0);
}
