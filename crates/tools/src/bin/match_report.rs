//! `match-report`: settings and report for a head-to-head match.
//!
//! ```text
//! match-report args   [--tc 10+0.1 | --nodes N] --games G --concurrency C --hash MB --openings FILE
//! match-report render [same settings] --log FILE --candidate REF --candidate-sha SHA [--candidate-bench N]
//!                     --baseline REF --baseline-sha SHA [--baseline-bench N]
//!                     --cpu MODEL --fastchess VERSION [--note TEXT]
//! ```
//!
//! `args` validates the settings and prints the fastchess arguments (minus
//! the two `-engine` blocks); invalid settings exit 2 with the reason.
//! `render` prints the Markdown report and writes a one-line headline to
//! stderr for the job log.

use std::process::ExitCode;

use rusty_chess_tools::matchup::{Meta, Settings, Side, fastchess_args, headline, render};
use rusty_chess_tools::sprt::parse_match;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("match-report: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut it = args.into_iter();
    let command = it.next().ok_or("expected `args` or `render`")?;
    let mut tc = "10+0.1".to_string();
    let mut nodes: Option<u64> = None;
    let mut games = 400u32;
    let mut concurrency = 4u32;
    let mut hash = 64u32;
    let mut openings = "tests/openings/openings.epd".to_string();
    let mut log = None;
    let mut cand = Side {
        label: "candidate".into(),
        sha: String::new(),
        signature: None,
    };
    let mut base = Side {
        label: "baseline".into(),
        sha: String::new(),
        signature: None,
    };
    let mut cpu = "unknown".to_string();
    let mut fastchess = "unknown".to_string();
    let mut note = String::new();
    let num = |flag: &str, v: &str| -> Result<u64, String> {
        v.trim()
            .parse::<u64>()
            .map_err(|_| format!("`{flag}` needs a whole number, got `{v}`"))
    };
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--tc" => tc = value.trim().to_string(),
            // An empty value means "not set", so a workflow can always pass it.
            "--nodes" if value.trim().is_empty() => nodes = None,
            "--nodes" => nodes = Some(num(&flag, &value)?),
            "--games" => games = num(&flag, &value)? as u32,
            "--concurrency" => concurrency = num(&flag, &value)? as u32,
            "--hash" => hash = num(&flag, &value)? as u32,
            "--openings" => openings = value,
            "--log" => log = Some(value),
            "--candidate" => cand.label = value,
            "--candidate-sha" => cand.sha = value,
            "--candidate-bench" => cand.signature = Some(num(&flag, &value)?),
            "--baseline" => base.label = value,
            "--baseline-sha" => base.sha = value,
            "--baseline-bench" => base.signature = Some(num(&flag, &value)?),
            "--cpu" => cpu = value,
            "--fastchess" => fastchess = value,
            "--note" => note = value,
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let settings = Settings::new(&tc, nodes, games, concurrency, hash, &openings)?;
    match command.as_str() {
        "args" => {
            println!("{}", fastchess_args(&settings).join(" "));
            Ok(())
        }
        "render" => {
            let log = log.ok_or("`--log` is required")?;
            let text = std::fs::read_to_string(&log).map_err(|e| format!("{log}: {e}"))?;
            let result = parse_match(&text).map_err(|e| format!("{log}: {e}"))?;
            let count = std::fs::read_to_string(&settings.openings)
                .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);
            let meta = Meta {
                candidate: cand,
                baseline: base,
                cpu,
                fastchess,
                openings: count,
                note,
            };
            eprintln!("{}", headline(&result, &meta));
            print!("{}", render(&result, &settings, &meta));
            Ok(())
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
