//! Benchmark history across releases.
//!
//! Each release records its numbers in three places, all written by
//! automation from the shipped build on a standardized runner:
//!
//! - a row in `docs/benchmarks.md`, the cumulative table (newest first);
//! - a `### Benchmarks` section under its own heading in `CHANGELOG.md`;
//! - a table with deltas against the previous release in its GitHub release
//!   notes.
//!
//! The first two are committed into the Release Please PR, so they are
//! reviewed before the release is cut and land on `main` with it.

use std::fmt::Write as _;

use crate::gate::Measured;

/// The marker line under which rows are kept in `docs/benchmarks.md`.
pub const TABLE_HEADER: &str = "| Version | Date | Bench signature | Search NPS | Perft NPS | Perft suite | Fail-high first | EBF | TT hit |";
const TABLE_RULE: &str = "|:--|:--|--:|--:|--:|:-:|--:|--:|--:|";

/// The headline numbers of one run, formatted for tables.
struct Cells {
    signature: String,
    search_nps: String,
    perft_nps: String,
    perft_suite: String,
    fh_first: String,
    ebf: String,
    tt_hit: String,
}

fn cells(m: &Measured) -> Cells {
    Cells {
        signature: m.signature.map_or("—".into(), fmt_int),
        search_nps: m
            .search_nps
            .filter(|&n| n > 0)
            .map_or("—".into(), |n| fmt_nps(n as f64)),
        perft_nps: m.perft_nps.map_or("—".into(), |n| fmt_nps(n as f64)),
        perft_suite: match (m.perft_failures, m.perft_positions) {
            (Some(0), Some(p)) => format!("pass ({p})"),
            (Some(f), _) => format!("{f} failed"),
            _ => "—".into(),
        },
        fh_first: m
            .fail_high_first
            .filter(|&f| f > 0.0)
            .map_or("—".into(), |f| format!("{:.1}%", f * 100.0)),
        ebf: m.ebf.map_or("—".into(), |e| format!("{e:.2}")),
        tt_hit: m
            .tt_hit
            .filter(|&t| t > 0.0)
            .map_or("—".into(), |t| format!("{:.1}%", t * 100.0)),
    }
}

/// The `docs/benchmarks.md` row for `version` (without a leading `v`).
pub fn row(version: &str, date: &str, m: &Measured) -> String {
    let c = cells(m);
    format!(
        "| v{version} | {date} | {} | {} | {} | {} | {} | {} | {} |",
        c.signature, c.search_nps, c.perft_nps, c.perft_suite, c.fh_first, c.ebf, c.tt_hit
    )
}

/// Insert `row` for `version` into the history document, newest first.
/// Returns `None` if the version already has a row (recording is
/// idempotent) or the table header is missing.
pub fn upsert_row(doc: &str, version: &str, row: &str) -> Option<String> {
    let prefix = format!("| v{version} |");
    if doc.lines().any(|l| l.starts_with(&prefix)) {
        return None;
    }
    let mut out = String::with_capacity(doc.len() + row.len() + 1);
    let mut inserted = false;
    let mut after_header = false;
    for line in doc.lines() {
        out.push_str(line);
        out.push('\n');
        if after_header && !inserted && line.starts_with("|:") {
            out.push_str(row);
            out.push('\n');
            inserted = true;
        }
        after_header = line == TABLE_HEADER;
    }
    inserted.then_some(out)
}

/// A fresh history document with an empty table.
pub fn empty_document() -> String {
    format!(
        "# Benchmark history\n\n\
Every release, measured on a standardized `ubuntu-latest` runner from the shipped build (`x86-64-v3`). \
Rows are written by the `Release benchmarks` workflow into the Release Please PR, so each one is reviewed before its release is cut. \
Each release's raw data (`bench.jsonl`, `stats.jsonl`, `perft.jsonl`, logs and `summary.json`) is attached to its [GitHub release](https://github.com/jwallace145/rusty-chess-v2/releases) as files and as one tarball, and kept in [benchmarks/](../benchmarks/) at each tag. \
The bench is 50 positions at a fixed depth with a fixed hash; the perft suite is `tests/perft.epd`. \
NPS varies between runner instances by up to 2x, so compare signatures and efficiency first and NPS only for large changes. \
What each column means is explained in [CONTRIBUTING.md](../CONTRIBUTING.md#coverage-suites-and-performance-bars).\n\n\
{TABLE_HEADER}\n{TABLE_RULE}\n"
    )
}

/// One release's headline numbers as JSON, attached to the release as
/// `summary.json` for historical analysis without parsing JSON lines.
pub fn summary_json(version: &str, date: &str, m: &Measured) -> String {
    let value = serde_json::json!({
        "v": 1,
        "version": format!("v{version}"),
        "date": date,
        "runner": "ubuntu-latest",
        "target_cpu": "x86-64-v3",
        "bench": {
            "positions": m.bench_config.map(|c| c.0),
            "depth": m.bench_config.map(|c| c.1),
            "hash_mb": m.bench_config.map(|c| c.2),
            "signature": m.signature,
            "nps": m.search_nps,
        },
        "efficiency": {
            "fail_high_first": m.fail_high_first,
            "ebf": m.ebf,
            "tt_hit": m.tt_hit,
        },
        "perft": {
            "positions": m.perft_positions,
            "max_depth": m.perft_max_depth,
            "failures": m.perft_failures,
            "nps": m.perft_nps,
        },
    });
    serde_json::to_string_pretty(&value).expect("summary serializes")
}

/// The `### Benchmarks` section for a version's changelog entry.
pub fn changelog_section(m: &Measured, prev: Option<(&str, &Measured)>) -> String {
    let c = cells(m);
    let mut out = String::new();
    let _ = writeln!(out, "### Benchmarks");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "* bench signature {}, search {} NPS, perft {} NPS ({}), fail-high first {}, EBF {}, TT hit {}",
        c.signature, c.search_nps, c.perft_nps, c.perft_suite, c.fh_first, c.ebf, c.tt_hit
    );
    if let Some((prev_version, p)) = prev {
        let mut deltas = Vec::new();
        if let (Some(a), Some(b)) = (p.fail_high_first, m.fail_high_first)
            && a > 0.0
        {
            deltas.push(format!("fail-high first {:+.1} pt", (b - a) * 100.0));
        }
        if let (Some(a), Some(b)) = (p.ebf, m.ebf) {
            deltas.push(format!("EBF {:+.2}", b - a));
        }
        if let (Some(a), Some(b)) = (p.search_nps, m.search_nps)
            && a > 0
        {
            deltas.push(format!(
                "search NPS {}",
                fmt_pct((b as f64 - a as f64) / a as f64 * 100.0)
            ));
        }
        if !deltas.is_empty() {
            let _ = writeln!(out, "* since v{prev_version}: {}", deltas.join(", "));
        }
    }
    let _ = writeln!(
        out,
        "* full history: [docs/benchmarks.md](docs/benchmarks.md)"
    );
    out
}

/// Insert `section` under the `## [version]` heading of `changelog`.
/// Returns `None` if the heading is missing or already has a section.
pub fn insert_into_changelog(changelog: &str, version: &str, section: &str) -> Option<String> {
    let heading = format!("## [{version}]");
    let start = changelog.find(&heading)?;
    let body_start = start + changelog[start..].find('\n')? + 1;
    let body_end = changelog[body_start..]
        .find("\n## [")
        .map_or(changelog.len(), |i| body_start + i + 1);
    if changelog[body_start..body_end].contains("### Benchmarks") {
        return None;
    }
    let mut out = String::with_capacity(changelog.len() + section.len() + 4);
    out.push_str(&changelog[..body_end]);
    if !out.ends_with("\n\n") {
        out.push('\n');
    }
    out.push_str(section);
    out.push('\n');
    out.push_str(&changelog[body_end..]);
    Some(out)
}

/// A release-notes row: name, this release, previous release, formatter, relative delta.
type NotesRow<'a> = (&'a str, Option<f64>, Option<f64>, fn(f64) -> String, bool);

/// The `## Benchmarks` section of a GitHub release's notes, with deltas
/// against the previous release.
pub fn release_notes(version: &str, m: &Measured, prev: Option<(&str, &Measured)>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Benchmarks");
    let _ = writeln!(out);
    let rows: [NotesRow; 6] = [
        (
            "Bench signature",
            m.signature.map(|v| v as f64),
            prev.and_then(|(_, p)| p.signature.map(|v| v as f64)),
            |v| fmt_int(v as u64),
            false,
        ),
        (
            "Search NPS",
            m.search_nps.map(|v| v as f64),
            prev.and_then(|(_, p)| p.search_nps.map(|v| v as f64)),
            fmt_nps,
            true,
        ),
        (
            "Perft NPS",
            m.perft_nps.map(|v| v as f64),
            prev.and_then(|(_, p)| p.perft_nps.map(|v| v as f64)),
            fmt_nps,
            true,
        ),
        (
            "Fail-high first",
            m.fail_high_first,
            prev.and_then(|(_, p)| p.fail_high_first),
            |v| format!("{:.1}%", v * 100.0),
            false,
        ),
        (
            "EBF",
            m.ebf,
            prev.and_then(|(_, p)| p.ebf),
            |v| format!("{v:.2}"),
            false,
        ),
        (
            "TT hit",
            m.tt_hit,
            prev.and_then(|(_, p)| p.tt_hit),
            |v| format!("{:.1}%", v * 100.0),
            false,
        ),
    ];
    let prev_label = prev.map_or("previous".to_string(), |(v, _)| format!("v{v}"));
    let _ = writeln!(out, "| | {prev_label} | v{version} | Δ |");
    let _ = writeln!(out, "|:--|--:|--:|--:|");
    for (name, now, before, fmt, relative) in rows {
        let Some(now) = now else { continue };
        let before_cell = before.map_or("—".into(), fmt);
        let delta = match before {
            Some(b) if b != 0.0 && relative => fmt_pct((now - b) / b * 100.0),
            Some(b) if name == "Bench signature" => {
                (if b == now { "same" } else { "changed" }).into()
            }
            Some(b) if name == "Fail-high first" || name == "TT hit" => {
                format!("{:+.1} pt", (now - b) * 100.0)
            }
            Some(b) if name == "EBF" => format!("{:+.2}", now - b),
            _ => String::new(),
        };
        let _ = writeln!(out, "| {name} | {before_cell} | {} | {delta} |", fmt(now));
    }
    if let (Some(f), Some(p)) = (m.perft_failures, m.perft_positions) {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "Perft suite: {} on {p} positions.",
            if f == 0 {
                "pass".to_string()
            } else {
                format!("{f} failures")
            }
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Measured on `ubuntu-latest` (`x86-64-v3`); NPS is not comparable across runner instances. Raw telemetry is attached below; the full history is in [docs/benchmarks.md](../../blob/main/docs/benchmarks.md)."
    );
    out
}

fn fmt_int(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

fn fmt_nps(v: f64) -> String {
    if v >= 1_000_000.0 {
        format!("{:.2}M", v / 1_000_000.0)
    } else if v >= 1_000.0 {
        format!("{:.0}k", v / 1_000.0)
    } else {
        format!("{v:.0}")
    }
}

fn fmt_pct(v: f64) -> String {
    if v >= 0.0 {
        format!("+{v:.1}%")
    } else {
        format!("−{:.1}%", -v)
    }
}
