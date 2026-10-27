//! `bench-history`: record a release's benchmarks.
//!
//! ```text
//! bench-history record --version 0.5.0 --date 2026-10-27 --dir run \
//!     [--prev-dir prev --prev-version 0.4.0] [--benchmarks docs/benchmarks.md] [--changelog CHANGELOG.md]
//! bench-history notes --version 0.5.0 --dir run [--prev-dir prev --prev-version 0.4.0]
//! bench-history summary --version 0.5.0 --date 2026-10-27 --dir run
//! ```
//!
//! `record` adds the version's row to the history document and its section
//! to the changelog, and prints `updated` or `unchanged`. `notes` prints the
//! release notes section. A directory holds `bench.jsonl`, `stats.jsonl`
//! and `perft.jsonl`; missing files are skipped.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::process::ExitCode;

use rusty_chess_search::telemetry::read_records;
use rusty_chess_tools::gate::Measured;
use rusty_chess_tools::history;

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bench-history: {e}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let mut it = args.into_iter();
    let command = it.next().ok_or("expected `record` or `notes`")?;
    let mut version = None;
    let mut date = None;
    let mut dir = None;
    let mut prev_dir = None;
    let mut prev_version = None;
    let mut benchmarks = "docs/benchmarks.md".to_string();
    let mut changelog = "CHANGELOG.md".to_string();
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--version" => version = Some(value.trim_start_matches('v').to_string()),
            "--date" => date = Some(value),
            "--dir" => dir = Some(value),
            "--prev-dir" => prev_dir = Some(value),
            "--prev-version" => prev_version = Some(value.trim_start_matches('v').to_string()),
            "--benchmarks" => benchmarks = value,
            "--changelog" => changelog = value,
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let version = version.ok_or("`--version` is required")?;
    let m = load(Path::new(&dir.ok_or("`--dir` is required")?))?;
    let prev = match (&prev_dir, &prev_version) {
        (Some(d), Some(v)) => Some((v.clone(), load(Path::new(d))?)),
        _ => None,
    };
    let prev_ref = prev.as_ref().map(|(v, m)| (v.as_str(), m));
    match command.as_str() {
        "notes" => {
            print!("{}", history::release_notes(&version, &m, prev_ref));
            Ok(())
        }
        "summary" => {
            let date = date.ok_or("`--date` is required")?;
            println!("{}", history::summary_json(&version, &date, &m));
            Ok(())
        }
        "record" => {
            let date = date.ok_or("`--date` is required")?;
            let doc =
                std::fs::read_to_string(&benchmarks).unwrap_or_else(|_| history::empty_document());
            let row = history::row(&version, &date, &m);
            let mut updated = false;
            if let Some(new_doc) = history::upsert_row(&doc, &version, &row) {
                std::fs::write(&benchmarks, new_doc).map_err(|e| format!("{benchmarks}: {e}"))?;
                updated = true;
            }
            if let Ok(text) = std::fs::read_to_string(&changelog) {
                let section = history::changelog_section(&m, prev_ref);
                if let Some(new_text) = history::insert_into_changelog(&text, &version, &section) {
                    std::fs::write(&changelog, new_text)
                        .map_err(|e| format!("{changelog}: {e}"))?;
                    updated = true;
                }
            }
            println!("{}", if updated { "updated" } else { "unchanged" });
            Ok(())
        }
        other => Err(format!("unknown command `{other}`")),
    }
}

fn load(dir: &Path) -> Result<Measured, String> {
    let mut m = Measured::default();
    for name in [
        "bench.jsonl",
        "stats.jsonl",
        "perft.jsonl",
        "instructions.jsonl",
    ] {
        let path = dir.join(name);
        match File::open(&path) {
            Ok(f) => m.absorb(
                &read_records(BufReader::new(f)).map_err(|e| format!("{}: {e}", path.display()))?,
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("{}: {e}", path.display())),
        }
    }
    Ok(m)
}
