//! `sprt-report`: the SPRT settings for fastchess, and the PR comment from
//! its log.
//!
//! ```text
//! sprt-report args --config perf-targets.toml [--timed] [--max-rounds N] [--concurrency N]
//! sprt-report render --config perf-targets.toml --log sprt.log --head SHA --base LABEL --fastchess VERSION --openings N
//! sprt-report combine --config perf-targets.toml --timed-log T --nodes-log N [--smoke] --head SHA --base LABEL --fastchess VERSION --openings N
//!                     [--timed-note TEXT] [--nodes-note TEXT]
//! ```
//!
//! `render` exits 1 unless H1 was accepted (a smoke test may also be
//! inconclusive). `combine` renders both runs of a dual SPRT and exits by the
//! timed run, the gate; a missing log is reported as not finished, and a
//! note (a spot interruption, no capacity) replaces a leg's result.
//! `--concurrency` overrides `[sprt] concurrency` for a runner with more
//! cores than the GitHub-hosted default.

use std::process::ExitCode;

use rusty_chess_tools::gate::Targets;
use rusty_chess_tools::sprt::{
    Meta, Notes, fastchess_args, parse, passes, render, render_dual_with_notes,
};

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
    let mut timed_log = None;
    let mut nodes_log = None;
    let mut max_rounds = None;
    let mut concurrency = None;
    let mut notes = Notes::default();
    let mut timed = false;
    let mut smoke = false;
    let mut meta = Meta {
        head_label: "PR".into(),
        base_label: "base".into(),
        fastchess: "unknown".into(),
        openings: 0,
    };
    while let Some(flag) = it.next() {
        if flag == "--timed" {
            timed = true;
            continue;
        }
        if flag == "--smoke" {
            smoke = true;
            continue;
        }
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--config" => config = Some(value),
            "--log" => log = Some(value),
            "--timed-log" => timed_log = Some(value),
            "--nodes-log" => nodes_log = Some(value),
            "--max-rounds" => {
                max_rounds = Some(
                    value
                        .parse()
                        .map_err(|_| format!("`--max-rounds` got `{value}`"))?,
                )
            }
            "--concurrency" => {
                concurrency = Some(
                    value
                        .parse()
                        .map_err(|_| format!("`--concurrency` got `{value}`"))?,
                )
            }
            // An empty note means none, so a workflow can always pass it.
            "--timed-note" => notes.timed = Some(value).filter(|v| !v.trim().is_empty()),
            "--nodes-note" => notes.nodes = Some(value).filter(|v| !v.trim().is_empty()),
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
    let mut targets =
        Targets::parse(&std::fs::read_to_string(&config).map_err(|e| format!("{config}: {e}"))?)
            .map_err(|e| format!("{config}: {e}"))?;
    if let Some(n) = max_rounds {
        targets.sprt.max_rounds = n;
    }
    if let Some(n) = concurrency {
        targets.sprt.concurrency = n;
    }
    targets.sprt.timed = timed;
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
            Ok(if passes(result.verdict, smoke) {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        "combine" => {
            // A log that is missing or unreadable means that job failed.
            let read = |path: Option<String>| {
                path.and_then(|p| std::fs::read_to_string(p).ok())
                    .and_then(|t| parse(&t).ok())
            };
            let timed_result = read(timed_log);
            let nodes_result = read(nodes_log);
            print!(
                "{}",
                render_dual_with_notes(
                    timed_result.as_ref(),
                    nodes_result.as_ref(),
                    &notes,
                    &targets.sprt,
                    &meta
                )
            );
            Ok(match timed_result {
                Some(r) if notes.timed.is_none() && passes(r.verdict, smoke) => ExitCode::SUCCESS,
                _ => ExitCode::FAILURE,
            })
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
