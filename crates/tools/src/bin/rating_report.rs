//! `rating-report`: the rating ladder's settings and report.
//!
//! ```text
//! rating-report levels --config perf-targets.toml
//! rating-report args --config perf-targets.toml [--games N]
//! rating-report render --config perf-targets.toml --logs DIR --version V --stockfish S --cpu C [--games N] --md FILE --json FILE
//! ```
//!
//! `render` reads `DIR/rating-<level>.log` for every configured level.

use std::process::ExitCode;

use rusty_chess_tools::gate::Targets;
use rusty_chess_tools::rating::{
    Level, Meta, estimate, fastchess_args, headline, render, render_json,
};
use rusty_chess_tools::sprt::parse_match;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rating-report: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut it = args.into_iter();
    let command = it.next().ok_or("expected `levels`, `args` or `render`")?;
    let (mut config, mut logs, mut md, mut json, mut games) = (None, None, None, None, None);
    let mut meta = Meta {
        version: "unknown".into(),
        stockfish: "Stockfish".into(),
        cpu: "unknown CPU".into(),
    };
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--config" => config = Some(value),
            "--logs" => logs = Some(value),
            "--md" => md = Some(value),
            "--json" => json = Some(value),
            "--games" => {
                games = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| format!("`--games` got `{value}`"))?,
                )
            }
            "--version" => meta.version = value,
            "--stockfish" => meta.stockfish = value,
            "--cpu" => meta.cpu = value,
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let config = config.ok_or("`--config` is required")?;
    let mut targets =
        Targets::parse(&std::fs::read_to_string(&config).map_err(|e| format!("{config}: {e}"))?)
            .map_err(|e| format!("{config}: {e}"))?;
    if let Some(g) = games {
        targets.rating.games_per_level = g;
    }
    let r = &targets.rating;
    match command.as_str() {
        "levels" => {
            println!(
                "{}",
                r.levels
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            Ok(())
        }
        "args" => {
            println!("{}", fastchess_args(r, r.games_per_level).join(" "));
            Ok(())
        }
        "render" => {
            let dir = logs.ok_or("`--logs` is required")?;
            let mut levels = Vec::new();
            for &elo in &r.levels {
                let path = format!("{dir}/rating-{elo}.log");
                let text = std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))?;
                levels.push(Level {
                    elo,
                    result: parse_match(&text).map_err(|e| format!("{path}: {e}"))?,
                });
            }
            let e = estimate(&levels);
            eprintln!("estimated rating: {}", headline(&e));
            let md_text = render(&levels, &e, r, &meta);
            match md {
                Some(path) => {
                    std::fs::write(&path, &md_text).map_err(|e| format!("{path}: {e}"))?
                }
                None => print!("{md_text}"),
            }
            if let Some(path) = json {
                std::fs::write(&path, render_json(&levels, &e, r, &meta))
                    .map_err(|e| format!("{path}: {e}"))?;
            }
            Ok(())
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
