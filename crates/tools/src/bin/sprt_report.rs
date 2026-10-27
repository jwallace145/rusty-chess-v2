//! `sprt-report`: the SPRT settings for fastchess, and the PR comment from
//! its log.
//!
//! ```text
//! sprt-report args --config perf-targets.toml
//! sprt-report render --config perf-targets.toml --log sprt.log --head SHA --base LABEL --fastchess VERSION --openings N
//! ```
//!
//! `render` exits 1 when H0 was accepted.

use std::process::ExitCode;

use rusty_chess_tools::gate::Targets;
use rusty_chess_tools::sprt::{Meta, Verdict, fastchess_args, parse, render};

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("sprt-report: {err}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let mut it = args.into_iter();
    let command = it.next().ok_or("expected `args` or `render`")?;
    let mut config = None;
    let mut log = None;
    let mut meta = Meta {
        head_label: "PR".into(),
        base_label: "base".into(),
        fastchess: "unknown".into(),
        openings: 0,
    };
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--config" => config = Some(value),
            "--log" => log = Some(value),
            "--head" => meta.head_label = value,
            "--base" => meta.base_label = value,
            "--fastchess" => meta.fastchess = value,
            "--openings" => {
                meta.openings = value
                    .parse()
                    .map_err(|_| format!("`--openings` got `{value}`"))?
            }
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let config = config.ok_or("`--config` is required")?;
    let targets =
        Targets::parse(&std::fs::read_to_string(&config).map_err(|e| format!("{config}: {e}"))?)
            .map_err(|e| format!("{config}: {e}"))?;
    match command.as_str() {
        "args" => {
            println!("{}", fastchess_args(&targets.sprt).join(" "));
            Ok(ExitCode::SUCCESS)
        }
        "render" => {
            let log = log.ok_or("`--log` is required")?;
            let text = std::fs::read_to_string(&log).map_err(|e| format!("{log}: {e}"))?;
            let result = parse(&text)?;
            print!("{}", render(&result, &targets.sprt, &meta));
            Ok(if result.verdict == Verdict::H0 {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            })
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
