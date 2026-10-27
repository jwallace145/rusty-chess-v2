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

    let mut rows = Vec::new();
    push_signature(&mut rows, base, head);
    push_numeric(
        &mut rows,
        "Search NPS",
        base.and_then(|b| b.search_nps).map(|v| v as f64),
        head.search_nps.map(|v| v as f64),
        fmt_nps,
        Better::Higher,
        speed,
    );
    push_numeric(
        &mut rows,
        "Perft NPS",
        base.and_then(|b| b.perft_nps).map(|v| v as f64),
        head.perft_nps.map(|v| v as f64),
        fmt_nps,
        Better::Higher,
        speed,
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
    out
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
