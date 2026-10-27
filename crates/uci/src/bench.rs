//! The deterministic bench.
//!
//! Searches every position in [`BENCH_POSITIONS`] to a fixed depth with a
//! fixed hash size and prints the total node count, the *signature*, and the
//! NPS. A changed signature means search behaviour changed; a changed NPS
//! with the same signature is a pure speed change.
//!
//! Phase 0 has no search, so every position reports zero nodes. The output
//! format, the JSON lines records and the CI job that uploads them are the
//! deliverable; the numbers arrive in Phase 2.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::Instant;

use rusty_chess_search::stats::SearchStats;
use rusty_chess_search::telemetry::{
    BenchRecord, JsonlWriter, Record, SCHEMA_VERSION, SummaryRecord,
};

use crate::cli::BenchArgs;
use crate::positions::BENCH_POSITIONS;

/// Run the bench, printing progress and the summary to `out` and writing JSON
/// lines to `args.out` if set. Returns the bench record.
pub fn run<W: Write>(args: &BenchArgs, out: &mut W) -> io::Result<BenchRecord> {
    let mut jsonl = match &args.out {
        Some(path) => Some(JsonlWriter::new(BufWriter::new(File::create(path)?))),
        None => None,
    };

    let mut stats = SearchStats::default();
    let mut total_nodes = 0u64;
    let started = Instant::now();

    for (i, fen) in BENCH_POSITIONS.iter().enumerate() {
        let position_started = Instant::now();
        // Phase 2 replaces this with a real search to `args.depth`.
        let (nodes, qnodes, bestmove, score_cp) = (0u64, 0u64, "0000", 0i32);
        let elapsed_ms = position_started.elapsed().as_millis() as u64;
        total_nodes += nodes;

        writeln!(out, "Position {}/{}: {fen}", i + 1, BENCH_POSITIONS.len())?;
        writeln!(
            out,
            "  nodes {nodes}  time {elapsed_ms} ms  (no search yet)"
        )?;

        if let Some(w) = jsonl.as_mut() {
            w.write(&Record::Summary(SummaryRecord {
                v: SCHEMA_VERSION,
                fen: (*fen).to_string(),
                depth: args.depth,
                nodes,
                qnodes,
                nps: nps(nodes + qnodes, elapsed_ms),
                elapsed_ms,
                bestmove: bestmove.to_string(),
                score_cp,
                stats: stats.summary(),
            }))?;
        }
    }

    let elapsed_ms = started.elapsed().as_millis() as u64;
    let record = BenchRecord {
        v: SCHEMA_VERSION,
        positions: BENCH_POSITIONS.len() as u32,
        depth: args.depth,
        hash_mb: args.hash_mb,
        nodes: total_nodes,
        nps: nps(total_nodes, elapsed_ms),
        elapsed_ms,
        stats: stats.summary(),
    };
    stats.reset();

    writeln!(out, "===========================")?;
    writeln!(out, "Positions       : {}", record.positions)?;
    writeln!(out, "Depth           : {}", record.depth)?;
    writeln!(out, "Hash (MB)       : {}", record.hash_mb)?;
    writeln!(out, "Total time (ms) : {}", record.elapsed_ms)?;
    writeln!(out, "Nodes searched  : {}", record.nodes)?;
    writeln!(out, "Nodes/second    : {}", record.nps)?;
    out.flush()?;

    if let Some(w) = jsonl.as_mut() {
        w.write(&Record::Bench(record.clone()))?;
    }
    Ok(record)
}

fn nps(nodes: u64, elapsed_ms: u64) -> u64 {
    if elapsed_ms == 0 {
        0
    } else {
        nodes * 1000 / elapsed_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fifty_well_formed_distinct_positions() {
        assert_eq!(BENCH_POSITIONS.len(), 50);
        let mut seen = std::collections::HashSet::new();
        for fen in BENCH_POSITIONS {
            assert!(seen.insert(fen), "duplicate position {fen}");
            let fields: Vec<&str> = fen.split(' ').collect();
            assert_eq!(fields.len(), 6, "{fen}");
            let ranks: Vec<&str> = fields[0].split('/').collect();
            assert_eq!(ranks.len(), 8, "{fen}");
            for rank in ranks {
                let squares: u32 = rank.chars().map(|c| c.to_digit(10).unwrap_or(1)).sum();
                assert_eq!(squares, 8, "{fen}");
            }
            assert!(matches!(fields[1], "w" | "b"), "{fen}");
            assert_eq!(fields[0].matches('K').count(), 1, "{fen}");
            assert_eq!(fields[0].matches('k').count(), 1, "{fen}");
        }
    }

    #[test]
    fn nps_handles_zero_time() {
        assert_eq!(nps(1000, 0), 0);
        assert_eq!(nps(1000, 500), 2000);
    }

    #[test]
    fn bench_without_output_file_prints_the_summary() {
        let mut out = Vec::new();
        let record = run(&BenchArgs::default(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(record.positions, 50);
        assert!(text.contains(&format!("Nodes searched  : {}", record.nodes)));
    }
}
