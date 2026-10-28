//! Hardware-independent speed measurement.
//!
//! Two ways around "the runner happened to be a faster chip":
//!
//! - **Paired runs.** Alternate bench runs of the base build and the PR
//!   build on the same machine and compare each pair. Whatever the machine's
//!   speed, it applies to both halves of a pair, so the ratio does not
//!   depend on it.
//! - **Instruction counts.** Run the bench under cachegrind and count the
//!   instructions executed. For a given binary that count is the same on
//!   every x86-64 machine, so it compares across runners and releases.

use std::path::Path;
use std::process::Command;

use rusty_chess_search::telemetry::{InstructionsRecord, SCHEMA_VERSION, SpeedRecord};

/// The summary of one bench run: nodes, nodes per second and wall time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchRun {
    /// Total nodes (the signature).
    pub nodes: u64,
    /// Nodes per second.
    pub nps: u64,
    /// Wall time in milliseconds.
    pub ms: u64,
}

/// The `Total time (ms)`, `Nodes searched` and `Nodes/second` lines of a
/// bench run's output.
pub fn parse_bench_run(text: &str) -> Option<BenchRun> {
    let field = |prefix: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(prefix))
            .and_then(|v| v.trim().parse::<u64>().ok())
    };
    Some(BenchRun {
        nodes: field("Nodes searched  :")?,
        nps: field("Nodes/second    :")?,
        ms: field("Total time (ms) :")?,
    })
}

/// The `Nodes searched` and `Nodes/second` lines of a bench run's output.
pub fn parse_bench_output(text: &str) -> Option<(u64, u64)> {
    parse_bench_run(text).map(|r| (r.nodes, r.nps))
}

/// The total instruction count from a cachegrind output file (`summary: N`).
pub fn parse_cachegrind_summary(text: &str) -> Option<u64> {
    text.lines()
        .find_map(|l| l.strip_prefix("summary:"))
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|v| v.parse().ok())
}

/// Mean and 95% confidence interval of the per-pair ratios `pr / base`.
pub fn paired_ratio(base: &[u64], pr: &[u64]) -> Option<(f64, f64, f64)> {
    if base.len() != pr.len() || base.is_empty() || base.contains(&0) {
        return None;
    }
    let ratios: Vec<f64> = base
        .iter()
        .zip(pr)
        .map(|(&b, &p)| p as f64 / b as f64)
        .collect();
    let n = ratios.len() as f64;
    let mean = ratios.iter().sum::<f64>() / n;
    if ratios.len() < 2 {
        return Some((mean, mean, mean));
    }
    let var = ratios.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let half = t_95(ratios.len() - 1) * (var / n).sqrt();
    Some((mean, mean - half, mean + half))
}

/// Two-sided 95% Student t critical values for small degrees of freedom.
fn t_95(df: usize) -> f64 {
    const T: [f64; 10] = [
        12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228,
    ];
    T.get(df.saturating_sub(1)).copied().unwrap_or(1.96)
}

fn bench(bin: &Path, depth: u32) -> Result<BenchRun, String> {
    let out = Command::new(bin)
        .args(["bench", "--depth", &depth.to_string()])
        .output()
        .map_err(|e| format!("{}: {e}", bin.display()))?;
    if !out.status.success() {
        return Err(format!(
            "{} bench failed: {}",
            bin.display(),
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    parse_bench_run(&String::from_utf8_lossy(&out.stdout))
        .ok_or_else(|| format!("{}: no bench summary", bin.display()))
}

/// Alternate `rounds` bench runs of `base` and `pr` (base first, then the
/// order swaps each round so drift over time affects both equally) after one
/// discarded warm-up run of each. Reports both nodes per second (per-node
/// speed, the right measure when the signature is unchanged) and time to
/// depth (the right measure when it changed).
pub fn measure_paired(
    base: &Path,
    pr: &Path,
    depth: u32,
    rounds: usize,
    cpu: Option<String>,
) -> Result<SpeedRecord, String> {
    bench(base, depth)?;
    bench(pr, depth)?;
    let mut base_runs = Vec::with_capacity(rounds);
    let mut pr_runs = Vec::with_capacity(rounds);
    for round in 0..rounds {
        let (b, p) = if round % 2 == 0 {
            let b = bench(base, depth)?;
            (b, bench(pr, depth)?)
        } else {
            let p = bench(pr, depth)?;
            (bench(base, depth)?, p)
        };
        base_runs.push(b);
        pr_runs.push(p);
    }
    let base_nps: Vec<u64> = base_runs.iter().map(|r| r.nps).collect();
    let pr_nps: Vec<u64> = pr_runs.iter().map(|r| r.nps).collect();
    let base_ms: Vec<u64> = base_runs.iter().map(|r| r.ms).collect();
    let pr_ms: Vec<u64> = pr_runs.iter().map(|r| r.ms).collect();
    let (ratio, ratio_low, ratio_high) =
        paired_ratio(&base_nps, &pr_nps).ok_or("no usable runs")?;
    // A sub-millisecond run has no usable time; then there is no time ratio.
    let time = paired_ratio(&base_ms, &pr_ms);
    Ok(SpeedRecord {
        v: SCHEMA_VERSION,
        depth,
        base_nps,
        pr_nps,
        ratio,
        ratio_low,
        ratio_high,
        cpu,
        base_ms,
        pr_ms,
        time_ratio: time.map(|t| t.0),
        time_ratio_low: time.map(|t| t.1),
        time_ratio_high: time.map(|t| t.2),
    })
}

/// Run `bin bench` under cachegrind (instruction counting only) and report
/// instructions per node.
pub fn measure_instructions(
    bin: &Path,
    depth: u32,
    out_file: &Path,
) -> Result<InstructionsRecord, String> {
    let out = Command::new("valgrind")
        .arg("--tool=cachegrind")
        .arg("--cache-sim=no")
        .arg(format!("--cachegrind-out-file={}", out_file.display()))
        .arg(bin)
        .args(["bench", "--depth", &depth.to_string()])
        .output()
        .map_err(|e| format!("valgrind: {e} (is it installed?)"))?;
    if !out.status.success() {
        return Err(format!(
            "valgrind failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    let nodes = parse_bench_run(&String::from_utf8_lossy(&out.stdout))
        .ok_or("no bench summary under valgrind")?
        .nodes;
    let text =
        std::fs::read_to_string(out_file).map_err(|e| format!("{}: {e}", out_file.display()))?;
    let instructions =
        parse_cachegrind_summary(&text).ok_or("no `summary:` line in the cachegrind output")?;
    Ok(InstructionsRecord {
        v: SCHEMA_VERSION,
        depth,
        nodes,
        instructions,
        per_node: instructions as f64 / nodes.max(1) as f64,
    })
}
