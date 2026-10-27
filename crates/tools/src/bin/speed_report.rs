//! `speed-report`: hardware-independent speed measurements, as JSON lines.
//!
//! ```text
//! speed-report paired --base BIN --pr BIN [--depth 6] [--rounds 5] [--cpu MODEL] --out speed.jsonl
//! speed-report instructions --bin BIN [--depth 6] --out instructions.jsonl
//! ```

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::process::ExitCode;

use rusty_chess_search::telemetry::{JsonlWriter, Record};
use rusty_chess_tools::speed::{measure_instructions, measure_paired};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("speed-report: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut it = args.into_iter();
    let command = it.next().ok_or("expected `paired` or `instructions`")?;
    let (mut base, mut pr, mut bin, mut out, mut cpu) = (None, None, None, None, None);
    let (mut depth, mut rounds) = (6u32, 5usize);
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--base" => base = Some(PathBuf::from(value)),
            "--pr" => pr = Some(PathBuf::from(value)),
            "--bin" => bin = Some(PathBuf::from(value)),
            "--out" => out = Some(PathBuf::from(value)),
            "--cpu" => cpu = Some(value),
            "--depth" => {
                depth = value
                    .parse()
                    .map_err(|_| format!("`--depth` got `{value}`"))?
            }
            "--rounds" => {
                rounds = value
                    .parse()
                    .map_err(|_| format!("`--rounds` got `{value}`"))?
            }
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let out = out.ok_or("`--out` is required")?;
    let record = match command.as_str() {
        "paired" => {
            let r = measure_paired(
                &base.ok_or("`--base` is required")?,
                &pr.ok_or("`--pr` is required")?,
                depth,
                rounds,
                cpu,
            )?;
            eprintln!(
                "PR speed vs base: {:+.1}% (95% CI {:+.1}% to {:+.1}%) over {} pairs",
                (r.ratio - 1.0) * 100.0,
                (r.ratio_low - 1.0) * 100.0,
                (r.ratio_high - 1.0) * 100.0,
                r.pr_nps.len()
            );
            Record::Speed(r)
        }
        "instructions" => {
            let bin = bin.ok_or("`--bin` is required")?;
            let cg = out.with_extension("cachegrind.out");
            let r = measure_instructions(&bin, depth, &cg)?;
            let _ = std::fs::remove_file(&cg);
            eprintln!(
                "{} instructions over {} nodes: {:.0} per node",
                r.instructions, r.nodes, r.per_node
            );
            Record::Instructions(r)
        }
        other => return Err(format!("unknown command `{other}`")),
    };
    let file = File::create(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    JsonlWriter::new(BufWriter::new(file))
        .write(&record)
        .map_err(|e| e.to_string())
}
