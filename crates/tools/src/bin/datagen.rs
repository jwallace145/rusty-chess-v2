//! `datagen`: self-play training data for NNUE.
//!
//! ```text
//! datagen --config datagen.toml --out DIR --run ID --task ID [--threads N] [--seed S]
//!         [--max-positions N] [--max-minutes M] [--stop-file PATH]
//!         [--engine-sha SHA] [--engine-bench N]
//! ```
//!
//! Plays games on `--threads` workers (default: every core) with the
//! settings in `[datagen]` of the config, and writes complete shards into
//! `--out` as they fill. Stops after `--max-positions` (about), after
//! `--max-minutes`, or when `--stop-file` appears; each worker finishes its
//! game and writes its last shard first. See `docs/datagen.md`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use rusty_chess_tools::datagen::{Provenance, Run, Settings, run};

fn main() -> ExitCode {
    match main_inner(std::env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("datagen: {e}");
            ExitCode::from(2)
        }
    }
}

fn main_inner(args: Vec<String>) -> Result<(), String> {
    let mut config = PathBuf::from("datagen.toml");
    let mut out = None;
    let mut run_id = None;
    let mut task = None;
    let mut threads = std::thread::available_parallelism().map_or(1, |n| n.get() as u32);
    let mut seed = 1u64;
    let mut max_positions = 0u64;
    let mut max_time = None;
    let mut stop_file = None;
    let mut engine_sha = String::from("unknown");
    let mut engine_bench = None;
    let mut it = args.into_iter();
    let num = |flag: &str, v: &str| {
        v.parse::<u64>()
            .map_err(|_| format!("`{flag}` needs a whole number, got `{v}`"))
    };
    while let Some(flag) = it.next() {
        let value = it.next().ok_or_else(|| format!("`{flag}` needs a value"))?;
        match flag.as_str() {
            "--config" => config = PathBuf::from(value),
            "--out" => out = Some(PathBuf::from(value)),
            "--run" => run_id = Some(value),
            "--task" => task = Some(value),
            "--threads" => threads = num(&flag, &value)? as u32,
            "--seed" => seed = num(&flag, &value)?,
            "--max-positions" => max_positions = num(&flag, &value)?,
            "--max-minutes" => max_time = Some(Duration::from_secs(num(&flag, &value)? * 60)),
            "--stop-file" => stop_file = Some(PathBuf::from(value)),
            "--engine-sha" => engine_sha = value,
            "--engine-bench" => engine_bench = Some(num(&flag, &value)?),
            other => return Err(format!("unknown option `{other}`")),
        }
    }
    let text =
        std::fs::read_to_string(&config).map_err(|e| format!("{}: {e}", config.display()))?;
    let settings = Settings::parse(&text).map_err(|e| format!("{}: {e}", config.display()))?;
    if threads == 0 {
        return Err("`--threads` must be at least 1".into());
    }
    let r = Run {
        settings,
        provenance: Provenance {
            run: run_id.ok_or("`--run` is required")?,
            task: task.ok_or("`--task` is required")?,
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            engine_sha,
            engine_bench,
        },
        threads,
        seed,
        out: out.ok_or("`--out` is required")?,
        max_positions,
        max_time,
        stop_file,
    };
    eprintln!(
        "datagen: run {} task {}, generation {}, {} nodes per move, {} threads, seed {}",
        r.provenance.run,
        r.provenance.task,
        r.settings.generation,
        r.settings.nodes,
        r.threads,
        r.seed
    );
    let t = run(&r, Duration::from_secs(30)).map_err(|e| e.to_string())?;
    eprintln!(
        "datagen: done: {} games, {} positions, {} shards",
        t.games, t.positions, t.shards
    );
    Ok(())
}
