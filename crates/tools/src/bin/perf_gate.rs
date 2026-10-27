//! `perf-gate`: compare a run's telemetry to the committed performance bars.
//!
//! ```text
//! perf-gate --targets perf-targets.toml --env ci [--enforce-noisy] [--ratchet-from OLD.toml] [FILE.jsonl ...]
//! ```
//!
//! Prints a Markdown table and exits 1 if an enforced bar is missed or the
//! targets were loosened. JSONL files that do not exist are skipped, so a
//! layer that is not measured yet simply shows as "not measured".

use std::fs::File;
use std::io::BufReader;
use std::process::ExitCode;

use rusty_chess_search::telemetry::read_records;
use rusty_chess_tools::gate::{Measured, Targets, evaluate};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("perf-gate: {err}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<bool, String> {
    let mut targets_path = None;
    let mut env = "local".to_string();
    let mut enforce_noisy = false;
    let mut ratchet_from = None;
    let mut files = Vec::new();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--targets" => targets_path = Some(it.next().ok_or("`--targets` needs a path")?),
            "--env" => env = it.next().ok_or("`--env` needs a value")?,
            "--enforce-noisy" => enforce_noisy = true,
            "--ratchet-from" => {
                ratchet_from = Some(it.next().ok_or("`--ratchet-from` needs a path")?)
            }
            other if other.starts_with("--") => return Err(format!("unknown option `{other}`")),
            file => files.push(file.to_string()),
        }
    }
    let targets_path = targets_path.ok_or("`--targets` is required")?;
    let targets =
        Targets::parse(&read(&targets_path)?).map_err(|e| format!("{targets_path}: {e}"))?;

    let mut ok = true;
    if let Some(old_path) = ratchet_from {
        let old = Targets::parse(&read(&old_path)?).map_err(|e| format!("{old_path}: {e}"))?;
        match targets.is_ratchet_from(&old) {
            Ok(()) => {
                println!("Targets ratchet check: no bar was loosened relative to `{old_path}`.\n")
            }
            Err(msg) => {
                println!("**Targets were loosened** relative to `{old_path}`: {msg}\n");
                ok = false;
            }
        }
    }

    let mut measured = Measured::default();
    for file in &files {
        match File::open(file) {
            Ok(f) => {
                let records =
                    read_records(BufReader::new(f)).map_err(|e| format!("{file}: {e}"))?;
                measured.absorb(&records);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("_`{file}` not found; that layer is not measured in this run._\n");
            }
            Err(e) => return Err(format!("{file}: {e}")),
        }
    }

    let outcome = evaluate(&targets, &measured, &env, enforce_noisy);
    print!("{outcome}");
    println!(
        "\nNoisy (speed) bars are {} in this run.",
        if enforce_noisy {
            "enforced"
        } else {
            "reported only"
        }
    );
    Ok(ok && outcome.passed())
}

fn read(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}
