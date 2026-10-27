//! JSON lines telemetry.
//!
//! One record per line, each tagged with a `type` and a schema version `v`,
//! so that artifacts from different commits stay comparable and can be
//! diffed by script. The same records are the shape of a future API
//! response.
//!
//! The example from the roadmap:
//!
//! ```json
//! {"type":"iter","v":1,"fen":"...","depth":9,"nodes":412331,"qnodes":198220,"nps":2310000,"ebf":2.1,"fh_first":0.91,"tt_hit":0.38,"score_cp":34,"pv":"e2e4 e7e5 ..."}
//! ```

use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};

use crate::stats::StatsSummary;

/// The schema version written into every record's `v` field. Bump it when a
/// field changes meaning or is removed; adding a field does not need a bump.
pub const SCHEMA_VERSION: u32 = 1;

/// A telemetry record. Serialises as a JSON object whose first key is `type`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Record {
    /// One iteration of iterative deepening.
    Iter(IterRecord),
    /// The result of one search.
    Summary(SummaryRecord),
    /// The result of a `bench` run over the fixed position set.
    Bench(BenchRecord),
    /// The result of a perft suite run.
    Perft(PerftRecord),
    /// Paired speed of a build against a base build on the same machine.
    Speed(SpeedRecord),
    /// Instructions executed by a bench run, counted by cachegrind.
    Instructions(InstructionsRecord),
}

/// One iteration of iterative deepening.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IterRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// The root position.
    pub fen: String,
    /// Iteration depth.
    pub depth: u32,
    /// Main-search nodes so far in this search.
    pub nodes: u64,
    /// Quiescence nodes so far in this search.
    pub qnodes: u64,
    /// Nodes per second so far in this search.
    pub nps: u64,
    /// Effective branching factor against the previous iteration, if any.
    pub ebf: Option<f64>,
    /// Fail-high-first rate, if the `stats` feature is on.
    pub fh_first: Option<f64>,
    /// Transposition table hit rate, if the `stats` feature is on.
    pub tt_hit: Option<f64>,
    /// Score in centipawns from the side to move.
    pub score_cp: i32,
    /// Principal variation in UCI notation, space separated.
    pub pv: String,
}

/// The result of one search.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SummaryRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// The root position.
    pub fen: String,
    /// Depth reached.
    pub depth: u32,
    /// Main-search nodes.
    pub nodes: u64,
    /// Quiescence nodes.
    pub qnodes: u64,
    /// Nodes per second.
    pub nps: u64,
    /// Wall-clock time.
    pub elapsed_ms: u64,
    /// Best move in UCI notation.
    pub bestmove: String,
    /// Score in centipawns from the side to move.
    pub score_cp: i32,
    /// Search efficiency counters, if the `stats` feature is on.
    pub stats: Option<StatsSummary>,
}

/// The result of a `bench` run. `nodes` is the signature: it changes only
/// when search behaviour changes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BenchRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// Positions searched.
    pub positions: u32,
    /// Depth each position was searched to.
    pub depth: u32,
    /// Hash size used.
    pub hash_mb: u32,
    /// Total nodes over every position: the bench signature.
    pub nodes: u64,
    /// Nodes per second over the whole run.
    pub nps: u64,
    /// Wall-clock time over the whole run.
    pub elapsed_ms: u64,
    /// Counters accumulated over the whole run, if the `stats` feature is on.
    pub stats: Option<StatsSummary>,
    /// The CPU model the run measured on; absent in records written before it existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu: Option<String>,
}

/// Search speed of a build relative to a base build, measured by
/// alternating bench runs of the two on the same machine, so the machine's
/// own speed cancels out. `ratio` is the mean of the per-pair NPS ratios
/// (PR / base); `ratio_low`..`ratio_high` is its 95% confidence interval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeedRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// Bench depth of every run.
    pub depth: u32,
    /// NPS of each base run, in run order.
    pub base_nps: Vec<u64>,
    /// NPS of each PR run, in run order.
    pub pr_nps: Vec<u64>,
    /// Mean of the per-pair ratios.
    pub ratio: f64,
    /// Lower end of the 95% confidence interval.
    pub ratio_low: f64,
    /// Upper end of the 95% confidence interval.
    pub ratio_high: f64,
    /// The CPU model both builds ran on.
    pub cpu: Option<String>,
}

/// Instructions a bench run executed, counted by cachegrind. Unlike time,
/// this is the same on every x86-64 machine for the same binary, so it is
/// comparable across runners and releases.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InstructionsRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// Bench depth.
    pub depth: u32,
    /// Nodes the bench searched (its signature at this depth).
    pub nodes: u64,
    /// Instructions executed, the whole process included.
    pub instructions: u64,
    /// `instructions / nodes`.
    pub per_node: f64,
}

/// The result of running the perft suite. `nps` is the move generation speed
/// (bulk counting); `failures` is the number of depth checks that did not
/// match the suite.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PerftRecord {
    /// Schema version, [`SCHEMA_VERSION`].
    pub v: u32,
    /// Positions in the suite.
    pub positions: u32,
    /// Deepest depth checked.
    pub max_depth: u32,
    /// Leaf nodes counted over the whole run.
    pub nodes: u64,
    /// Wall-clock time over the whole run.
    pub elapsed_ms: u64,
    /// Leaf nodes per second over the whole run.
    pub nps: u64,
    /// Depth checks whose count did not match, or that failed to run.
    pub failures: u32,
}

/// Writes [`Record`]s as JSON lines. Each `write` ends with a newline and a
/// flush so that a partial run still leaves complete lines behind.
pub struct JsonlWriter<W: Write> {
    out: W,
}

impl<W: Write> JsonlWriter<W> {
    /// Wrap a writer.
    pub fn new(out: W) -> Self {
        Self { out }
    }

    /// Write one record as one line.
    pub fn write(&mut self, record: &Record) -> io::Result<()> {
        serde_json::to_writer(&mut self.out, record)?;
        self.out.write_all(b"\n")?;
        self.out.flush()
    }

    /// Give the underlying writer back.
    pub fn into_inner(self) -> W {
        self.out
    }
}

/// Read every record from a JSON lines stream. Blank lines are skipped; any
/// other line that is not a record is an error.
pub fn read_records<R: BufRead>(reader: R) -> io::Result<Vec<Record>> {
    let mut records = Vec::new();
    for (n, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record = serde_json::from_str(&line).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("line {}: {e}", n + 1))
        })?;
        records.push(record);
    }
    Ok(records)
}
