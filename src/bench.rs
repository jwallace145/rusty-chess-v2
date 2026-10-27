//! The deterministic bench.
//!
//! Searches every position in [`BENCH_POSITIONS`] to a fixed depth and
//! prints the total node count, the *signature*, and the NPS. A changed
//! signature means search behaviour changed; a changed NPS with the same
//! signature is a pure speed change. With the `stats` feature the counters
//! accumulate over the whole run and go into the final record.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{Limits, Searcher};
use rusty_chess_search::telemetry::{
    BenchRecord, JsonlWriter, Record, SCHEMA_VERSION, SummaryRecord,
};

use crate::cli::BenchArgs;
use crate::positions::BENCH_POSITIONS;

/// Run the bench, printing progress and the summary to `out` and writing JSON
/// lines to `args.out` if set. Returns the bench record.
pub fn run<W: Write>(args: &BenchArgs, out: &mut W) -> io::Result<BenchRecord> {
    rusty_chess_core::attacks::init();
    let mut jsonl = match &args.out {
        Some(path) => Some(JsonlWriter::new(BufWriter::new(File::create(path)?))),
        None => None,
    };
    let limits = Limits {
        depth: Some(args.depth),
        ..Limits::default()
    };
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    searcher.set_hash_mb(args.hash_mb as usize);
    let mut total_nodes = 0u64;
    let started = Instant::now();

    for (i, fen) in BENCH_POSITIONS.iter().enumerate() {
        let position = Position::from_fen(fen)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        let mut board = Board::from_position(position);
        searcher.clear_hash(); // every position is searched cold, so the signature is reproducible
        let mut iterations = Vec::new();
        let result = searcher.search(&mut board, &limits, &mut |it| {
            iterations.push(Record::Iter(it.clone()))
        });
        let nodes = result.nodes + result.qnodes;
        let elapsed_ms = result.elapsed.as_millis() as u64;
        total_nodes += nodes;

        writeln!(out, "Position {}/{}: {fen}", i + 1, BENCH_POSITIONS.len())?;
        writeln!(
            out,
            "  bestmove {} score {} nodes {nodes} time {elapsed_ms} ms",
            result.best, result.score
        )?;

        if let Some(w) = jsonl.as_mut() {
            for record in &iterations {
                w.write(record)?;
            }
            w.write(&Record::Summary(SummaryRecord {
                v: SCHEMA_VERSION,
                fen: (*fen).to_string(),
                depth: result.depth,
                nodes: result.nodes,
                qnodes: result.qnodes,
                nps: nps(nodes, elapsed_ms),
                elapsed_ms,
                bestmove: result.best.to_string(),
                score_cp: result.score,
                stats: None,
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
        stats: searcher.stats().summary(),
    };

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
    (nodes * 1000).checked_div(elapsed_ms).unwrap_or(0)
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
            Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"));
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
        let args = BenchArgs {
            depth: 1,
            ..BenchArgs::default()
        };
        let record = run(&args, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(record.positions, 50);
        assert!(record.nodes > 0);
        assert!(text.contains(&format!("Nodes searched  : {}", record.nodes)));
    }
}
