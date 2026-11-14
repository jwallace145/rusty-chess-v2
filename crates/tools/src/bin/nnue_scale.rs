//! `nnue-scale`: what an evaluation thinks each piece is worth.
//!
//! ```text
//! nnue-scale --fens FILE [--net NET | --pesto]
//! ```
//!
//! For each position in FILE (one FEN or EPD line each), removes every piece
//! but the kings in turn and reads the static evaluation again; prints, for
//! each kind of piece, how many were measured and the mean, the median and
//! the 10th and 90th percentiles of what the evaluation lost. With `--net`
//! the evaluation is that net file's, read by the engine's own loader; with
//! `--pesto` the hand-written evaluation's; with neither, the embedded
//! default net's.
//!
//! The search's margins are written in these units, and a net's units move
//! from net to net (`docs/research-log.md`, "The net's material scale"), so
//! a net's scale is worth recording beside its loss. `--pesto` is the check:
//! it prints PeSTO's own table values.

use std::process::ExitCode;
use std::sync::Arc;

use braveheart_search::nnue::{Network, default_network};
use braveheart_tools::scale::{KINDS, piece_values, summarize};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("nnue-scale: {e}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut net_path = None;
    let mut fens_path = None;
    let mut pesto = false;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        match flag.as_str() {
            "--pesto" => pesto = true,
            "--net" | "--fens" => {
                let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
                if flag == "--net" {
                    net_path = Some(value.clone());
                } else {
                    fens_path = Some(value.clone());
                }
            }
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    if pesto && net_path.is_some() {
        return Err("--pesto and --net exclude each other".into());
    }
    braveheart_core::attacks::init();
    let (net, label) = if pesto {
        (None, "PeSTO".to_string())
    } else if let Some(path) = net_path {
        let bytes = std::fs::read(&path).map_err(|e| format!("{path}: {e}"))?;
        let net = Network::from_bytes(&bytes).map_err(|e| format!("{path}: {e}"))?;
        (Some(Arc::new(net)), path)
    } else {
        (
            Some(default_network()),
            format!("{} (embedded)", braveheart_search::nnue::DEFAULT_NET_ID),
        )
    };
    let fens_path = fens_path.ok_or("--fens is required")?;
    let text = std::fs::read_to_string(&fens_path).map_err(|e| format!("{fens_path}: {e}"))?;
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    let values = piece_values(lines.iter().copied(), net.as_ref());
    println!(
        "nnue-scale: {label}, {} positions from {fens_path}",
        lines.len()
    );
    println!("piece    samples     mean   median      p10      p90");
    for (kind, values) in KINDS.iter().zip(values.iter()) {
        let name = format!("{kind:?}").to_lowercase();
        match summarize(values) {
            Some(s) => println!(
                "{name:8} {:7} {:8.1} {:8} {:8} {:8}",
                s.samples, s.mean, s.median, s.p10, s.p90
            ),
            None => println!("{name:8}       0        -        -        -        -"),
        }
    }
    Ok(())
}
