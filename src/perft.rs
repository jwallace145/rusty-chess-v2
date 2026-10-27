//! The `perft` subcommand: divide one position, or run the EPD suite and
//! measure move generation speed.

use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::time::Instant;

use rusty_chess_core::movegen::{perft, perft_divide};
use rusty_chess_core::position::Position;
use rusty_chess_search::telemetry::{JsonlWriter, Record};
use rusty_chess_tools::perft::{Suite, run as run_suite};

use crate::cli::PerftArgs;

/// Run perft. Returns `Ok(false)` if the suite had failures.
pub fn run<W: Write>(args: &PerftArgs, out: &mut W) -> io::Result<bool> {
    rusty_chess_core::attacks::init();
    match &args.suite {
        Some(path) => suite(args, path, out),
        None => divide(args, out).map(|()| true),
    }
}

fn divide<W: Write>(args: &PerftArgs, out: &mut W) -> io::Result<()> {
    let fen = args
        .fen
        .as_deref()
        .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let mut pos = Position::from_fen(fen)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let started = Instant::now();
    let divide = perft_divide(&mut pos, args.depth);
    let nodes: u64 = divide.iter().map(|(_, n)| n).sum();
    let elapsed_ms = started.elapsed().as_millis() as u64;
    for (m, n) in &divide {
        writeln!(out, "{m}: {n}")?;
    }
    writeln!(out, "===========================")?;
    writeln!(out, "Depth           : {}", args.depth)?;
    writeln!(out, "Total time (ms) : {elapsed_ms}")?;
    writeln!(out, "Nodes searched  : {nodes}")?;
    writeln!(out, "Nodes/second    : {}", nps(nodes, elapsed_ms))?;
    out.flush()
}

fn suite<W: Write>(args: &PerftArgs, path: &std::path::Path, out: &mut W) -> io::Result<bool> {
    let text = std::fs::read_to_string(path)?;
    let suite = Suite::parse(&text).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: {e}", path.display()),
        )
    })?;
    let max_depth = args.max_depth.unwrap_or_else(|| suite.max_depth());
    let report = run_suite(&suite, max_depth, |fen, depth| {
        let mut pos = Position::from_fen(fen).map_err(|e| e.to_string())?;
        Ok::<u64, String>(perft(&mut pos, depth))
    });
    for failure in report.failures() {
        match &failure.got {
            Some(got) => writeln!(
                out,
                "FAIL {} D{}: expected {}, got {got}",
                failure.fen, failure.depth, failure.expected
            )?,
            None => writeln!(
                out,
                "FAIL {} D{}: {}",
                failure.fen,
                failure.depth,
                failure.error.as_deref().unwrap_or("error")
            )?,
        }
    }
    let record = report.record(max_depth);
    writeln!(out, "===========================")?;
    writeln!(out, "Positions       : {}", record.positions)?;
    writeln!(out, "Max depth       : {}", record.max_depth)?;
    writeln!(
        out,
        "Checks          : {} passed, {} failed",
        report.checks.len() - report.failures().len(),
        report.failures().len()
    )?;
    writeln!(out, "Total time (ms) : {}", record.elapsed_ms)?;
    writeln!(out, "Nodes searched  : {}", record.nodes)?;
    writeln!(out, "Nodes/second    : {}", record.nps)?;
    out.flush()?;
    if let Some(path) = &args.out {
        JsonlWriter::new(BufWriter::new(File::create(path)?)).write(&Record::Perft(record))?;
    }
    Ok(report.passed())
}

fn nps(nodes: u64, elapsed_ms: u64) -> u64 {
    (nodes * 1000).checked_div(elapsed_ms).unwrap_or(0)
}
