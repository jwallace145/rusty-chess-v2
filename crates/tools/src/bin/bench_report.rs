//! `bench-report`: render the PR bench comment.
//!
//! ```text
//! bench-report --targets perf-targets.toml --head DIR --head-label SHA
//!              [--base DIR --base-label main@SHA] [--gate gate.md] [--gate-passed true|false] [--noisy-enforced]
//! ```
//!
//! `DIR` holds a run's `bench.jsonl`, `stats.jsonl` and `perft.jsonl`; files
//! that do not exist are skipped. Prints Markdown to stdout.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::ExitCode;

use rusty_chess_search::telemetry::read_records;
use rusty_chess_tools::gate::{Measured, Targets};
use rusty_chess_tools::report::{Meta, render};

const FILES: [&str; 5] = [
    "bench.jsonl",
    "stats.jsonl",
    "perft.jsonl",
    "instructions.jsonl",
    "speed.jsonl",
];

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(md) => {
            print!("{md}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("bench-report: {err}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<String, String> {
    let mut targets_path = None;
    let mut head = None;
    let mut base = None;
    let mut meta = Meta {
        base_label: None,
        head_label: "PR".into(),
        gate_passed: None,
        gate_table: None,
        noisy_enforced: false,
    };
    let mut it = args.into_iter();
    let next = |flag: &str, it: &mut std::vec::IntoIter<String>| {
        it.next().ok_or_else(|| format!("`{flag}` needs a value"))
    };
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--targets" => targets_path = Some(next("--targets", &mut it)?),
            "--head" => head = Some(next("--head", &mut it)?),
            "--head-label" => meta.head_label = next("--head-label", &mut it)?,
            "--base" => base = Some(next("--base", &mut it)?),
            "--base-label" => meta.base_label = Some(next("--base-label", &mut it)?),
            "--gate" => {
                let path = next("--gate", &mut it)?;
                let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                // Keep only the table from perf-gate's output.
                let table: Vec<&str> = text.lines().filter(|l| l.starts_with('|')).collect();
                meta.gate_table = Some(table.join("\n"));
            }
            "--gate-passed" => {
                meta.gate_passed = Some(match next("--gate-passed", &mut it)?.as_str() {
                    "true" => true,
                    "false" => false,
                    other => return Err(format!("`--gate-passed` got `{other}`")),
                });
            }
            "--noisy-enforced" => meta.noisy_enforced = true,
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let targets_path = targets_path.ok_or("`--targets` is required")?;
    let targets = Targets::parse(
        &std::fs::read_to_string(&targets_path).map_err(|e| format!("{targets_path}: {e}"))?,
    )
    .map_err(|e| format!("{targets_path}: {e}"))?;
    let head_dir = head.ok_or("`--head` is required")?;
    let head = load(Path::new(&head_dir))?;
    let base = match base {
        Some(dir) => Some(load(Path::new(&dir))?),
        None => None,
    };
    if base.is_none() {
        meta.base_label = None;
    }
    Ok(render(base.as_ref(), &head, &targets, &meta))
}

fn load(dir: &Path) -> Result<Measured, String> {
    let mut measured = Measured::default();
    for name in FILES {
        let path = dir.join(name);
        match File::open(&path) {
            Ok(f) => {
                let records = read_records(BufReader::new(f))
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                measured.absorb(&records);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Ok(measured)
}
