//! The perft suite: an EPD file of positions with expected leaf-node counts
//! per depth, and a runner that checks a perft implementation against it and
//! measures its speed.
//!
//! Suite format, one position per line:
//!
//! ```text
//! <fen> ;D1 <nodes> ;D2 <nodes> ...
//! ```
//!
//! Blank lines and lines starting with `#` are ignored. The suite is the
//! move generator's coverage set: it grows by appending lines, and every line
//! stays forever.

use std::fmt;
use std::time::{Duration, Instant};

use rusty_chess_search::telemetry::{PerftRecord, SCHEMA_VERSION};

/// One suite position with its expected counts, in increasing depth order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The position.
    pub fen: String,
    /// `(depth, expected leaf nodes)` pairs.
    pub expected: Vec<(u32, u64)>,
}

/// A parsed suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suite {
    entries: Vec<Entry>,
}

/// A line that could not be parsed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    line: usize,
    message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for ParseError {}

impl Suite {
    /// Parse the EPD text.
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        let mut entries = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let err = |message: String| ParseError {
                line: i + 1,
                message,
            };
            let mut parts = line.split(';').map(str::trim);
            let fen = parts.next().unwrap_or_default().to_string();
            check_fen(&fen).map_err(|m| err(format!("bad fen `{fen}`: {m}")))?;
            let mut expected = Vec::new();
            for op in parts.filter(|p| !p.is_empty()) {
                let (depth, nodes) = op
                    .split_once(char::is_whitespace)
                    .ok_or_else(|| err(format!("expected `D<depth> <nodes>`, got `{op}`")))?;
                let depth: u32 = depth
                    .strip_prefix('D')
                    .and_then(|d| d.parse().ok())
                    .ok_or_else(|| err(format!("expected `D<depth>`, got `{depth}`")))?;
                let nodes: u64 = nodes
                    .trim()
                    .parse()
                    .map_err(|_| err(format!("expected a node count, got `{nodes}`")))?;
                expected.push((depth, nodes));
            }
            if expected.is_empty() {
                return Err(err("no depth counts".to_string()));
            }
            expected.sort_unstable();
            entries.push(Entry { fen, expected });
        }
        Ok(Self { entries })
    }

    /// The positions in file order.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The deepest depth any entry expects.
    pub fn max_depth(&self) -> u32 {
        self.entries
            .iter()
            .flat_map(|e| e.expected.iter().map(|&(d, _)| d))
            .max()
            .unwrap_or(0)
    }
}

/// A structural FEN check: six fields, eight ranks of eight squares, one king
/// each, and a side to move. Legality is the move generator's business.
fn check_fen(fen: &str) -> Result<(), &'static str> {
    let fields: Vec<&str> = fen.split_whitespace().collect();
    if fields.len() != 6 {
        return Err("expected six fields");
    }
    let ranks: Vec<&str> = fields[0].split('/').collect();
    if ranks.len() != 8 {
        return Err("expected eight ranks");
    }
    for rank in ranks {
        let mut squares = 0;
        for c in rank.chars() {
            match c {
                '1'..='8' => squares += c.to_digit(10).unwrap(),
                'p' | 'n' | 'b' | 'r' | 'q' | 'k' | 'P' | 'N' | 'B' | 'R' | 'Q' | 'K' => {
                    squares += 1
                }
                _ => return Err("unexpected character in the piece placement"),
            }
        }
        if squares != 8 {
            return Err("a rank does not have eight squares");
        }
    }
    if fields[0].matches('K').count() != 1 || fields[0].matches('k').count() != 1 {
        return Err("expected exactly one king per side");
    }
    if !matches!(fields[1], "w" | "b") {
        return Err("side to move must be `w` or `b`");
    }
    Ok(())
}

/// One depth check of one position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Check {
    /// The position.
    pub fen: String,
    /// The depth checked.
    pub depth: u32,
    /// What the suite expects.
    pub expected: u64,
    /// What perft returned, or `None` if it failed.
    pub got: Option<u64>,
    /// The error, if perft failed.
    pub error: Option<String>,
    /// How long this check took.
    pub elapsed: Duration,
}

impl Check {
    /// Whether the count matched.
    pub fn ok(&self) -> bool {
        self.got == Some(self.expected)
    }
}

/// The result of running a suite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Every depth check, in suite order.
    pub checks: Vec<Check>,
    /// Leaf nodes counted in total (the sum of what perft returned).
    pub nodes: u64,
    /// Wall-clock time in total.
    pub elapsed: Duration,
    positions: u32,
}

impl Report {
    /// The checks that did not match or did not run.
    pub fn failures(&self) -> Vec<&Check> {
        self.checks.iter().filter(|c| !c.ok()).collect()
    }

    /// Whether every check matched.
    pub fn passed(&self) -> bool {
        self.checks.iter().all(Check::ok)
    }

    /// Leaf nodes per second over the whole run.
    pub fn nps(&self) -> u64 {
        let ms = self.elapsed.as_millis() as u64;
        (self.nodes * 1000).checked_div(ms).unwrap_or(0)
    }

    /// The telemetry record for this run. `max_depth` is the depth limit the
    /// run was given, so that records from different limits are not compared.
    pub fn record(&self, max_depth: u32) -> PerftRecord {
        PerftRecord {
            v: SCHEMA_VERSION,
            positions: self.positions,
            max_depth,
            nodes: self.nodes,
            elapsed_ms: self.elapsed.as_millis() as u64,
            nps: self.nps(),
            failures: self.failures().len() as u32,
        }
    }
}

/// Run every check in the suite up to `max_depth` with `perft`, which takes a
/// FEN and a depth and returns the leaf-node count.
pub fn run<F, E>(suite: &Suite, max_depth: u32, mut perft: F) -> Report
where
    F: FnMut(&str, u32) -> Result<u64, E>,
    E: fmt::Display,
{
    let started = Instant::now();
    let mut checks = Vec::new();
    let mut nodes = 0u64;
    for entry in suite.entries() {
        for &(depth, expected) in entry.expected.iter().filter(|&&(d, _)| d <= max_depth) {
            let check_started = Instant::now();
            let (got, error) = match perft(&entry.fen, depth) {
                Ok(n) => (Some(n), None),
                Err(e) => (None, Some(e.to_string())),
            };
            nodes += got.unwrap_or(0);
            checks.push(Check {
                fen: entry.fen.clone(),
                depth,
                expected,
                got,
                error,
                elapsed: check_started.elapsed(),
            });
        }
    }
    Report {
        checks,
        nodes,
        elapsed: started.elapsed(),
        positions: suite.entries().len() as u32,
    }
}
