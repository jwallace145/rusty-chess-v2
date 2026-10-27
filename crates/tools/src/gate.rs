//! The performance gate.
//!
//! `perf-targets.toml` commits the minimum bars for each measured layer:
//! move generation speed, search speed, and search efficiency. The gate reads
//! the telemetry records a run produced, compares them to the bars for the
//! environment it is running in, and reports.
//!
//! Speed bars are *noisy*: shared CI runners vary from run to run, so they
//! are reported on every PR and enforced only where the caller asks (release
//! PRs, local runs). Efficiency bars are deterministic, so they are always
//! enforced. Bars only ratchet up: a PR that lowers one fails the ratchet
//! check.

use std::fmt;

use rusty_chess_search::telemetry::Record;
use serde::Deserialize;

/// A bar with a value per environment. An environment the file does not
/// name gets `0`, which means "not enforced".
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct PerEnv {
    /// A quiet local machine.
    pub local: u64,
    /// Shared CI runners.
    pub ci: u64,
}

impl PerEnv {
    /// The bar for `env`.
    pub fn get(&self, env: &str) -> u64 {
        match env {
            "local" => self.local,
            "ci" => self.ci,
            _ => 0,
        }
    }
}

/// Move generation bars.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Movegen {
    /// Minimum bulk-counting perft leaf nodes per second.
    pub min_perft_nps: PerEnv,
}

/// Search speed bars.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Search {
    /// Minimum bench nodes per second.
    pub min_nps: PerEnv,
}

/// Search efficiency bars; deterministic, so not per environment.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Efficiency {
    /// Minimum share of fail-highs caused by the first move searched.
    pub min_fail_high_first: f64,
    /// Maximum effective branching factor. `0` means not enforced.
    pub max_ebf: f64,
}

/// Noise thresholds for the PR bench comment: a delta smaller than these is
/// shown as unchanged rather than as an improvement or a regression.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(default)]
pub struct Comment {
    /// Percent, for the speed rows (NPS), which shared runners measure noisily.
    pub speed_noise_pct: f64,
    /// Percentage points for fail-high-first, percent for EBF.
    pub efficiency_noise_pct: f64,
}

impl Default for Comment {
    fn default() -> Self {
        Self {
            speed_noise_pct: 10.0,
            efficiency_noise_pct: 1.0,
        }
    }
}

/// The committed bars.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Targets {
    /// Move generation.
    pub movegen: Movegen,
    /// Search speed.
    pub search: Search,
    /// Search efficiency.
    pub efficiency: Efficiency,
    /// PR comment noise thresholds.
    #[serde(default)]
    pub comment: Comment,
}

impl Targets {
    /// Parse `perf-targets.toml`.
    pub fn parse(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }

    /// Check that no bar in `self` is looser than in `old`. A bar that was
    /// `0` (not enforced) in `old` may take any value.
    pub fn is_ratchet_from(&self, old: &Self) -> Result<(), String> {
        let mut loosened = Vec::new();
        let mut min = |name: &str, new: f64, old: f64| {
            if new < old {
                loosened.push(format!("{name} lowered from {old} to {new}"));
            }
        };
        for env in ["local", "ci"] {
            min(
                &format!("movegen.min_perft_nps.{env}"),
                self.movegen.min_perft_nps.get(env) as f64,
                old.movegen.min_perft_nps.get(env) as f64,
            );
            min(
                &format!("search.min_nps.{env}"),
                self.search.min_nps.get(env) as f64,
                old.search.min_nps.get(env) as f64,
            );
        }
        min(
            "efficiency.min_fail_high_first",
            self.efficiency.min_fail_high_first,
            old.efficiency.min_fail_high_first,
        );
        if old.efficiency.max_ebf != 0.0
            && (self.efficiency.max_ebf == 0.0 || self.efficiency.max_ebf > old.efficiency.max_ebf)
        {
            loosened.push(format!(
                "efficiency.max_ebf raised from {} to {}",
                old.efficiency.max_ebf, self.efficiency.max_ebf
            ));
        }
        if loosened.is_empty() {
            Ok(())
        } else {
            Err(loosened.join("; "))
        }
    }
}

/// What a run measured, gathered from its telemetry records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measured {
    /// Perft leaf nodes per second, from the last `perft` record.
    pub perft_nps: Option<u64>,
    /// Perft depth checks that failed, from the last `perft` record.
    pub perft_failures: Option<u32>,
    /// Positions in the perft suite, from the last `perft` record.
    pub perft_positions: Option<u32>,
    /// Deepest perft depth checked, from the last `perft` record.
    pub perft_max_depth: Option<u32>,
    /// The bench signature (total nodes), from the last `bench` record.
    pub signature: Option<u64>,
    /// Bench nodes per second, from the last `bench` record.
    pub search_nps: Option<u64>,
    /// Bench `(positions, depth, hash_mb)`, from the last `bench` record.
    pub bench_config: Option<(u32, u32, u32)>,
    /// Fail-high-first rate, from the last `bench` record with stats.
    pub fail_high_first: Option<f64>,
    /// Effective branching factor, from the last `iter` record that has one.
    pub ebf: Option<f64>,
}

impl Measured {
    /// Take measurements from `records`; later records win.
    pub fn absorb(&mut self, records: &[Record]) {
        for record in records {
            match record {
                Record::Perft(p) => {
                    self.perft_nps = Some(p.nps);
                    self.perft_failures = Some(p.failures);
                    self.perft_positions = Some(p.positions);
                    self.perft_max_depth = Some(p.max_depth);
                }
                Record::Bench(b) => {
                    self.signature = Some(b.nodes);
                    self.search_nps = Some(b.nps);
                    self.bench_config = Some((b.positions, b.depth, b.hash_mb));
                    if let Some(stats) = &b.stats {
                        self.fail_high_first = Some(stats.fh_first);
                    }
                }
                Record::Iter(i) => {
                    if i.ebf.is_some() {
                        self.ebf = i.ebf;
                    }
                }
                Record::Summary(_) => {}
            }
        }
    }
}

/// One bar compared to one measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct GateCheck {
    /// `section.bar`, as in the targets file.
    pub name: String,
    /// What was measured, if anything.
    pub measured: Option<f64>,
    /// The bar.
    pub target: f64,
    /// Whether the bar was met (or not enforced).
    pub ok: bool,
    /// Whether the bar is a speed bar that shared runners measure noisily.
    pub noisy: bool,
    /// Whether the bar was `0` and therefore not enforced.
    pub skipped: bool,
}

/// The gate's verdict.
#[derive(Clone, Debug, PartialEq)]
pub struct Outcome {
    /// Every bar, in targets-file order.
    pub checks: Vec<GateCheck>,
    /// Whether noisy bars count against the verdict.
    pub enforce_noisy: bool,
    /// The environment the bars were taken from.
    pub env: String,
}

impl Outcome {
    /// Whether the gate passes: every enforced bar is met.
    pub fn passed(&self) -> bool {
        self.checks
            .iter()
            .all(|c| c.ok || (c.noisy && !self.enforce_noisy))
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "| Bar | Measured | Target ({}) | Result |", self.env)?;
        writeln!(f, "| --- | ---: | ---: | --- |")?;
        for c in &self.checks {
            let measured = c.measured.map_or("not measured".to_string(), fmt_num);
            let result = if c.skipped {
                "not enforced"
            } else if c.ok {
                "pass"
            } else if c.noisy && !self.enforce_noisy {
                "below bar (noisy, reported only)"
            } else {
                "FAIL"
            };
            writeln!(
                f,
                "| `{}` | {measured} | {} | {result} |",
                c.name,
                fmt_num(c.target)
            )?;
        }
        Ok(())
    }
}

fn fmt_num(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() >= 1000.0 {
        let digits = format!("{}", v as u64);
        let mut out = String::new();
        for (i, ch) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push('_');
            }
            out.push(ch);
        }
        out
    } else {
        format!("{v}")
    }
}

/// Compare `measured` to the bars in `targets` for `env`.
pub fn evaluate(targets: &Targets, measured: &Measured, env: &str, enforce_noisy: bool) -> Outcome {
    let min = |name: &str, value: Option<f64>, target: f64, noisy: bool| GateCheck {
        name: name.to_string(),
        measured: value,
        target,
        ok: target == 0.0 || value.is_some_and(|v| v >= target),
        noisy,
        skipped: target == 0.0,
    };
    let max = |name: &str, value: Option<f64>, target: f64| GateCheck {
        name: name.to_string(),
        measured: value,
        target,
        ok: target == 0.0 || value.is_some_and(|v| v <= target),
        noisy: false,
        skipped: target == 0.0,
    };
    Outcome {
        checks: vec![
            min(
                "movegen.min_perft_nps",
                measured.perft_nps.map(|v| v as f64),
                targets.movegen.min_perft_nps.get(env) as f64,
                true,
            ),
            min(
                "search.min_nps",
                measured.search_nps.map(|v| v as f64),
                targets.search.min_nps.get(env) as f64,
                true,
            ),
            min(
                "efficiency.min_fail_high_first",
                measured.fail_high_first,
                targets.efficiency.min_fail_high_first,
                false,
            ),
            max(
                "efficiency.max_ebf",
                measured.ebf,
                targets.efficiency.max_ebf,
            ),
        ],
        enforce_noisy,
        env: env.to_string(),
    }
}
