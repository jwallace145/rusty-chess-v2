//! `nnue-eval`: the engine's static evaluation of positions with a network.
//!
//! ```text
//! nnue-eval --net NET --fens FILE
//! nnue-eval --net NET --check eval_check.json [--tolerance 50] [--relative 0.05] [--mean 20]
//! ```
//!
//! Loads NET with the engine's own loader (`crates/search/src/nnue.rs`) and
//! prints, for each FEN in FILE, its evaluation in centipawns for the side to
//! move, the same number the search uses. With `--check`, compares each with
//! the trainer's float evaluation of the same position (`trainer_cp` in the
//! trainer's `eval_check.json`) and exits 1 when any position differs by more
//! than the larger of `--tolerance` centipawns and `--relative` of its size,
//! or the mean difference exceeds `--mean`. The difference is quantization
//! error (QA = 255, QB = 64): on the first pilot net, an independent
//! implementation reproduced the trainer's float output exactly from
//! `raw.bin` and the engine's integer output exactly from the net, and the two
//! differed by a mean of 9 cp, at most 28. A layout, perspective or scale
//! mismatch is hundreds (swapping the output weights' halves, a perspective
//! bug: mean 740), so the bars sit well between.

use std::process::ExitCode;
use std::sync::Arc;

use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::Network;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("nnue-eval: {e}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut net = None;
    let mut fens = None;
    let mut check = None;
    let mut tolerance = 50.0f64;
    let mut relative = 0.05f64;
    let mut mean_max = 20.0f64;
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--net" => net = Some(value.clone()),
            "--fens" => fens = Some(value.clone()),
            "--check" => check = Some(value.clone()),
            "--relative" => {
                relative = value
                    .parse()
                    .map_err(|_| format!("--relative: `{value}`"))?
            }
            "--mean" => mean_max = value.parse().map_err(|_| format!("--mean: `{value}`"))?,
            "--tolerance" => {
                tolerance = value
                    .parse()
                    .map_err(|_| format!("--tolerance: `{value}`"))?
            }
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    rusty_chess_core::attacks::init();
    let net_path = net.ok_or("--net is required")?;
    let bytes = std::fs::read(&net_path).map_err(|e| format!("{net_path}: {e}"))?;
    let net = Arc::new(Network::from_bytes(&bytes).map_err(|e| format!("{net_path}: {e}"))?);
    let eval = |fen: &str| -> Result<i32, String> {
        let pos = Position::from_fen(fen).map_err(|e| format!("{fen}: {e}"))?;
        let mut b = Board::from_position(pos);
        b.set_network(Some(net.clone()));
        Ok(b.evaluate())
    };
    if let Some(path) = check {
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
        let entries: Vec<serde_json::Value> =
            serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
        let mut worst = 0.0f64;
        let mut sum = 0.0f64;
        let mut over = 0usize;
        for e in &entries {
            let fen = e["fen"].as_str().ok_or("an entry without `fen`")?;
            let want = e["trainer_cp"]
                .as_f64()
                .ok_or("an entry without `trainer_cp`")?;
            let got = eval(fen)? as f64;
            let diff = (got - want).abs();
            sum += diff;
            if diff > tolerance.max(relative * want.abs()) {
                over += 1;
                eprintln!("nnue-eval: {fen}: engine {got} cp, trainer {want:.1} cp");
            }
            if diff > worst {
                worst = diff;
            }
        }
        let n = entries.len().max(1) as f64;
        let mean = sum / n;
        println!(
            "nnue-eval: {} positions; engine against trainer: mean |diff| {mean:.2} cp (bar {mean_max}), max {worst:.2} cp; beyond max({tolerance} cp, {:.0}%): {over}",
            entries.len(),
            relative * 100.0
        );
        return Ok(if over == 0 && mean <= mean_max && !entries.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        });
    }
    let fens_path = fens.ok_or("--fens or --check is required")?;
    for fen in std::fs::read_to_string(&fens_path)
        .map_err(|e| format!("{fens_path}: {e}"))?
        .lines()
    {
        if !fen.trim().is_empty() {
            println!("{} | {fen}", eval(fen)?);
        }
    }
    Ok(ExitCode::SUCCESS)
}
