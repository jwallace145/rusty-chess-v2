//! `openings`: write the strength-test openings file.
//!
//! ```text
//! openings [--count N] [--plies N] [--depth N] [--max-cp N] [--seed N] [--out FILE]
//! ```

use std::io::Write;
use std::process::ExitCode;

use rusty_chess_tools::openings::{Settings, generate};

fn main() -> ExitCode {
    let mut settings = Settings::default();
    let mut out = None;
    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let value = match args.next() {
            Some(v) => v,
            None => {
                eprintln!("openings: `{flag}` needs a value");
                return ExitCode::from(2);
            }
        };
        let parsed = |v: &str| v.parse::<u64>().map_err(|_| format!("`{flag}` got `{v}`"));
        let result: Result<(), String> = match flag.as_str() {
            "--count" => parsed(&value).map(|n| settings.count = n as usize),
            "--plies" => parsed(&value).map(|n| settings.plies = n as u32),
            "--depth" => parsed(&value).map(|n| settings.depth = n as u32),
            "--max-cp" => parsed(&value).map(|n| settings.max_cp = n as i32),
            "--seed" => parsed(&value).map(|n| settings.seed = n),
            "--out" => {
                out = Some(value.clone());
                Ok(())
            }
            other => Err(format!("unknown option `{other}`")),
        };
        if let Err(e) = result {
            eprintln!("openings: {e}");
            return ExitCode::from(2);
        }
    }
    let fens = generate(&settings);
    let text = fens.join("\n") + "\n";
    match out {
        Some(path) => {
            if let Err(e) = std::fs::write(&path, text) {
                eprintln!("openings: {path}: {e}");
                return ExitCode::FAILURE;
            }
            eprintln!("wrote {} openings to {path}", fens.len());
        }
        None => {
            let _ = std::io::stdout().write_all(text.as_bytes());
        }
    }
    ExitCode::SUCCESS
}
