//! The PR bench comment: one compact Markdown table comparing the PR's run
//! with the latest release (or, failing that, the latest run on the base
//! branch), a one-line gate verdict, and the details folded away.
//!
//! Rows appear only for what was measured on at least one side, so the
//! comment grows with the engine instead of starting bloated. Each row ends
//! with a marker: 🟢 improved or 🔴 regressed beyond the noise threshold in
//! `perf-targets.toml`, ⚪ unchanged or within noise, 🆕 no baseline.

use std::fmt::Write as _;

use crate::gate::{Measured, Targets};

/// Labels and context for the comment.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// How the baseline is identified: a release tag such as `v0.1.0`, or
    /// `main@2c0c213` when falling back to the base branch; `None` if there is none.
    pub base_label: Option<String>,
    /// How the PR run is identified, e.g. the head SHA.
    pub head_label: String,
    /// The performance gate's verdict for this run, if it ran.
    pub gate_passed: Option<bool>,
    /// The gate's Markdown table, folded into the details.
    pub gate_table: Option<String>,
    /// Whether the gate enforced the noisy (speed) bars in this run.
    pub noisy_enforced: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Marker {
    Better,
    Worse,
    Same,
    New,
}

impl Marker {
    fn glyph(self) -> &'static str {
        match self {
            Marker::Better => "🟢",
            Marker::Worse => "🔴",
            Marker::Same => "⚪",
            Marker::New => "🆕",
        }
    }
}

/// Whether a higher value is better, with the noise threshold as a percent of
/// the base value (`Relative`) or as an absolute difference (`Absolute`).
#[derive(Clone, Copy)]
enum Better {
    Higher,
    Lower,
}

#[derive(Clone, Copy)]
enum Noise {
    Relative(f64),
    Absolute(f64),
    /// Shown with its delta but never marked better or worse: the number
    /// depends on which machine measured it.
    Informational,
}

struct Row {
    name: &'static str,
    base: String,
    head: String,
    delta: String,
    marker: Marker,
}

/// Render the comment.
pub fn render(base: Option<&Measured>, head: &Measured, targets: &Targets, meta: &Meta) -> String {
    let speed = Noise::Relative(targets.comment.speed_noise_pct);
    let eff_pct = Noise::Relative(targets.comment.efficiency_noise_pct);
    let eff_pt = Noise::Absolute(targets.comment.efficiency_noise_pct / 100.0);

    let _ = speed; // absolute NPS rows are informational; speed is judged by the paired row
    let mut rows = Vec::new();
    push_signature(&mut rows, base, head);
    push_paired(&mut rows, head, targets.comment.paired_noise_pct);
    push_numeric(
        &mut rows,
        "Instructions / node",
        base.and_then(|b| b.instructions_per_node),
        head.instructions_per_node,
        |v| format!("{v:.0}"),
        Better::Lower,
        Noise::Relative(targets.comment.instructions_noise_pct),
    );
    push_numeric(
        &mut rows,
        "Search NPS",
        base.and_then(|b| b.search_nps).map(|v| v as f64),
        head.search_nps.map(|v| v as f64),
        fmt_nps,
        Better::Higher,
        Noise::Informational,
    );
    push_numeric(
        &mut rows,
        "Perft NPS",
        base.and_then(|b| b.perft_nps).map(|v| v as f64),
        head.perft_nps.map(|v| v as f64),
        fmt_nps,
        Better::Higher,
        Noise::Informational,
    );
    push_perft_suite(&mut rows, base, head);
    push_numeric(
        &mut rows,
        "Fail-high first",
        base.and_then(|b| b.fail_high_first),
        head.fail_high_first,
        fmt_pct,
        Better::Higher,
        eff_pt,
    );
    push_numeric(
        &mut rows,
        "EBF",
        base.and_then(|b| b.ebf),
        head.ebf,
        fmt_ebf,
        Better::Lower,
        eff_pct,
    );

    let base_col = meta.base_label.as_deref().unwrap_or("base");
    let mut out = String::new();
    let _ = writeln!(out, "### 📊 Bench report");
    let _ = writeln!(out);
    let _ = writeln!(out, "| | `{base_col}` | PR | Δ | |");
    let _ = writeln!(out, "|:--|--:|--:|--:|:-:|");
    for r in &rows {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            r.name,
            r.base,
            r.head,
            r.delta,
            r.marker.glyph()
        );
    }
    if rows.is_empty() {
        let _ = writeln!(out, "| _nothing measured_ | | | | |");
    }
    let _ = writeln!(out);
    match meta.gate_passed {
        Some(true) => {
            let _ = writeln!(
                out,
                "Performance gate: **pass** · speed bars {}",
                if meta.noisy_enforced {
                    "enforced"
                } else {
                    "reported only on PRs"
                }
            );
        }
        Some(false) => {
            let _ = writeln!(out, "Performance gate: **fail** · see the details below");
        }
        None => {}
    }
    if meta.base_label.is_none() {
        let _ = writeln!(
            out,
            "_no baseline: no release or base-branch Bench run to compare with yet_"
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "<details><summary>Details</summary>");
    let _ = writeln!(out);
    if let Some((positions, depth, hash)) = head.bench_config {
        let _ = writeln!(
            out,
            "- Bench: {positions} positions, depth {depth}, hash {hash} MB"
        );
    }
    if let (Some(positions), Some(depth)) = (head.perft_positions, head.perft_max_depth) {
        let _ = writeln!(out, "- Perft: {positions} positions to depth {depth}");
    }
    let _ = writeln!(
        out,
        "- Base: {} · PR: `{}`",
        meta.base_label
            .as_deref()
            .map_or("none".to_string(), |l| format!("`{l}`")),
        meta.head_label
    );
    let _ = writeln!(
        out,
        "- Deltas under ±{}% (speed) or ±{:.1} pt (efficiency) are shown as unchanged; shared runners are noisy",
        fmt_f(targets.comment.speed_noise_pct),
        targets.comment.efficiency_noise_pct
    );
    if let Some(table) = &meta.gate_table {
        let _ = writeln!(out);
        let _ = write!(out, "{}", table.trim_end());
        let _ = writeln!(out);
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "</details>");

    // What each shown row means, for a reader who has not seen the roadmap.
    let _ = writeln!(out);
    let _ = writeln!(out, "<details><summary>What these numbers mean</summary>");
    let _ = writeln!(out);
    for r in &rows {
        let _ = writeln!(out, "- **{}**: {}", r.name, explain(r.name));
    }
    let _ = writeln!(
        out,
        "- **Markers**: 🟢 improved or 🔴 regressed beyond the noise threshold, ⚪ unchanged or within it, 🆕 nothing to compare with. Thresholds and the bars the gate enforces live in `perf-targets.toml`; speed bars are enforced on release PRs, efficiency bars always, and a bar may never be lowered."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "</details>");
    out
}

/// One sentence per row, for the explainer section.
fn explain(name: &str) -> &'static str {
    match name {
        "Bench signature" => {
            "total nodes searched over the fixed bench positions (`src/positions.rs`) at a fixed depth and hash size. Deterministic: a different number means search behaviour changed (`feat`); the same number with a different NPS is a pure speed change (`perf`)."
        }
        "Speed vs base" => {
            "the PR's bench NPS divided by its merge base's, from alternating runs of both builds on the same runner, with the 95% interval. The runner's own speed cancels out, so this is the speed number to trust; marked only when the whole interval clears the threshold."
        }
        "Instructions / node" => {
            "instructions executed per bench node, counted by cachegrind. The same on every x86-64 machine for the same binary, so it compares exactly against the release; lower is cheaper per node (a change that searches fewer nodes may still raise it)."
        }
        "Search NPS" => {
            "nodes per second of that bench on whichever runner each side ran on. Informational only: runners differ by up to 2x; see Speed vs base."
        }
        "Perft NPS" => {
            "leaf nodes per second over the perft suite (`tests/perft.epd`) with bulk counting: raw move generation and make/unmake speed. Informational only, for the same reason as Search NPS."
        }
        "Perft suite" => {
            "every position and depth in `tests/perft.epd` must match its known leaf-node count; a failure is a move generation bug."
        }
        "Fail-high first" => {
            "share of beta cutoffs produced by the first move searched, from the `--features stats` build. Move ordering quality; the Phase 3 gate is 90%."
        }
        "EBF" => {
            "effective branching factor between iterative-deepening iterations: how many moves per node the search effectively looks at. Lower is better; the Phase 4 target is about 2."
        }
        _ => "",
    }
}

fn push_signature(rows: &mut Vec<Row>, base: Option<&Measured>, head: &Measured) {
    let b = base.and_then(|b| b.signature);
    let h = head.signature;
    if b.is_none() && h.is_none() {
        return;
    }
    let (delta, marker) = match (b, h) {
        (Some(b), Some(h)) if b == h => ("same".to_string(), Marker::Same),
        (Some(_), Some(_)) => ("changed".to_string(), Marker::Same),
        _ => (String::new(), Marker::New),
    };
    rows.push(Row {
        name: "Bench signature",
        base: b.map_or("—".into(), fmt_int),
        head: h.map_or("—".into(), fmt_int),
        delta,
        marker,
    });
}

fn push_perft_suite(rows: &mut Vec<Row>, base: Option<&Measured>, head: &Measured) {
    let b = base.and_then(|b| b.perft_failures);
    let h = head.perft_failures;
    if b.is_none() && h.is_none() {
        return;
    }
    let show = |f: Option<u32>| match f {
        None => "—".to_string(),
        Some(0) => "pass".to_string(),
        Some(1) => "1 failure".to_string(),
        Some(n) => format!("{n} failures"),
    };
    let marker = match h {
        Some(0) => Marker::Same,
        Some(_) => Marker::Worse,
        None => Marker::Same,
    };
    rows.push(Row {
        name: "Perft suite",
        base: show(b),
        head: show(h),
        delta: String::new(),
        marker,
    });
}

#[allow(clippy::too_many_arguments)]
/// The paired speed row: the PR against its base build on the same runner.
/// Marked only when the whole 95% interval clears the noise threshold.
fn push_paired(rows: &mut Vec<Row>, head: &Measured, noise_pct: f64) {
    let Some((ratio, low, high)) = head.speed else {
        return;
    };
    let pct = |r: f64| (r - 1.0) * 100.0;
    let marker = if pct(low) >= noise_pct {
        Marker::Better
    } else if pct(high) <= -noise_pct {
        Marker::Worse
    } else {
        Marker::Same
    };
    rows.push(Row {
        name: "Speed vs base",
        base: "same runner".into(),
        head: fmt_signed_pct(pct(ratio)),
        delta: format!("±{:.1}%", (pct(high) - pct(low)) / 2.0),
        marker,
    });
}

fn push_numeric(
    rows: &mut Vec<Row>,
    name: &'static str,
    base: Option<f64>,
    head: Option<f64>,
    fmt: fn(f64) -> String,
    better: Better,
    noise: Noise,
) {
    if base.is_none() && head.is_none() {
        return;
    }
    let (delta, marker) = match (base, head) {
        // A baseline of zero is "not measured yet" (the layer did not exist), not a value to compare with.
        (Some(b), Some(h)) if b == 0.0 && h != 0.0 => (String::new(), Marker::New),
        (Some(b), Some(h)) => {
            let (delta, beyond) = match noise {
                Noise::Relative(pct) => {
                    let rel = if b == 0.0 { 0.0 } else { (h - b) / b * 100.0 };
                    (fmt_signed_pct(rel), rel.abs() >= pct)
                }
                Noise::Absolute(abs) => {
                    let diff = h - b;
                    (fmt_signed_pt(diff * 100.0), diff.abs() >= abs - 1e-12)
                }
                Noise::Informational => {
                    let rel = if b == 0.0 { 0.0 } else { (h - b) / b * 100.0 };
                    (fmt_signed_pct(rel), false)
                }
            };
            let improved = match better {
                Better::Higher => h > b,
                Better::Lower => h < b,
            };
            let marker = if !beyond {
                Marker::Same
            } else if improved {
                Marker::Better
            } else {
                Marker::Worse
            };
            (delta, marker)
        }
        _ => (String::new(), Marker::New),
    };
    rows.push(Row {
        name,
        base: base.map_or("—".into(), fmt),
        head: head.map_or("—".into(), fmt),
        delta,
        marker,
    });
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
    format!("{:.1}%", v * 100.0)
}

fn fmt_ebf(v: f64) -> String {
    format!("{v:.2}")
}

fn fmt_f(v: f64) -> String {
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

fn fmt_signed_pct(v: f64) -> String {
    if v == 0.0 {
        "0.0%".into()
    } else if v > 0.0 {
        format!("+{v:.1}%")
    } else {
        format!("−{:.1}%", -v)
    }
}

fn fmt_signed_pt(v: f64) -> String {
    if v == 0.0 {
        "0.0 pt".into()
    } else if v > 0.0 {
        format!("+{v:.1} pt")
    } else {
        format!("−{:.1} pt", -v)
    }
}
