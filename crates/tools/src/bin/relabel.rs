//! `relabel`: re-score stored self-play positions with another engine,
//! keeping every original label (`docs/relabel.md`).
//!
//! ```text
//! relabel --in DIR --out DIR --net ID [--net-file FILE] [--nodes 5000] [--threads N]
//!         [--label-id ID] [--engine-sha SHA]
//! ```
//!
//! `--in` is a local copy of `selfplay/v1/` or any part of it (shards are
//! found by their `manifests/`). Each shard's relabelled records go to
//! `--out` under the same `gen=/run=/task=` partition, `positions/` and
//! `manifests/` (the games are unchanged and not copied). A shard whose
//! relabelled manifest exists is skipped, so a run can resume. `--net` is the
//! embedded net's id, `pesto`, or a retired net's id with `--net-file`: its
//! file from the dataset bucket (`nnue::NET_STORE`), checked against the
//! SHA-256 in `nnue::RETIRED_NETS`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use rusty_chess_search::nnue::{NETS, Network, network_by_id, retired_net, retired_net_message};
use rusty_chess_search::search::Searcher;
use rusty_chess_tools::relabel::{RelabelStats, relabel_shard};
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A shard file's text, gzip-compressed (`.gz`) or plain.
fn read_text(dir: &Path, stem: &str, ext: &str) -> Result<String, String> {
    let gz = dir.join(format!("{stem}.{ext}.gz"));
    let plain = dir.join(format!("{stem}.{ext}"));
    let mut text = String::new();
    if gz.exists() {
        GzDecoder::new(std::fs::File::open(&gz).map_err(|e| format!("{}: {e}", gz.display()))?)
            .read_to_string(&mut text)
            .map_err(|e| format!("{}: {e}", gz.display()))?;
    } else {
        text = std::fs::read_to_string(&plain).map_err(|e| format!("{}: {e}", plain.display()))?;
    }
    Ok(text)
}

/// Write `bytes` to `path` through a temporary name, so a file is whole or
/// absent.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Every manifest under `dir`, sorted.
fn manifests(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json")
                && p.parent().is_some_and(|q| q.ends_with("manifests"))
            {
                out.push(p);
            }
        }
    }
    out.sort();
    out
}

struct Job<'a> {
    input: &'a Path,
    output: &'a Path,
    net: Option<Arc<Network>>,
    net_id: &'a str,
    net_sha: String,
    nodes: u64,
    label_id: &'a str,
    engine_sha: &'a str,
}

/// Relabel the shard whose manifest is `manifest`. Returns its statistics,
/// or `None` when it was already done.
fn relabel_one(
    job: &Job,
    manifest: &Path,
    searcher: &mut Searcher,
) -> Result<Option<RelabelStats>, String> {
    let rel = manifest.strip_prefix(job.input).unwrap();
    let out_manifest = job.output.join(rel);
    if out_manifest.exists() {
        return Ok(None);
    }
    let task_dir = manifest.parent().unwrap().parent().unwrap();
    let stem = manifest.file_stem().unwrap().to_string_lossy().to_string();
    let mut m: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(manifest).map_err(|e| format!("{}: {e}", manifest.display()))?,
    )
    .map_err(|e| format!("{}: {e}", manifest.display()))?;
    let positions = read_text(&task_dir.join("positions"), &stem, "txt")?;
    let games = read_text(&task_dir.join("games"), &stem, "jsonl")?;
    let source_sha = hex(positions.as_bytes());
    if let Some(want) = m["sha256"][0].as_str()
        && want != source_sha
    {
        return Err(format!(
            "{stem}: positions SHA-256 {source_sha} is not the manifest's {want}"
        ));
    }
    if let Some(hash) = m["settings"]["hash_mb"].as_u64() {
        searcher.set_hash_mb(hash as usize);
    }
    let started = Instant::now();
    let (records, stats) = relabel_shard(&positions, &games, searcher, job.net.clone(), job.nodes)
        .map_err(|e| format!("{stem}: {e}"))?;
    if Some(stats.records) != m["positions"].as_u64() {
        return Err(format!(
            "{stem}: relabelled {} records, the manifest says {}",
            stats.records, m["positions"]
        ));
    }
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(records.as_bytes())
        .map_err(|e| e.to_string())?;
    let out_task = job.output.join(rel.parent().unwrap().parent().unwrap());
    write_atomic(
        &out_task.join("positions").join(format!("{stem}.txt.gz")),
        &gz.finish().map_err(|e| e.to_string())?,
    )?;
    m["sha256"][0] = serde_json::Value::String(hex(records.as_bytes()));
    m["relabel"] = serde_json::json!({
        "label_id": job.label_id,
        "net": {"id": job.net_id, "sha256": job.net_sha},
        "nodes": job.nodes,
        "engine_version": env!("CARGO_PKG_VERSION"),
        "engine_sha": job.engine_sha,
        "source_positions_sha256": source_sha,
        "stats": stats,
        "seconds": started.elapsed().as_secs_f64(),
    });
    write_atomic(
        &out_manifest,
        serde_json::to_string_pretty(&m).unwrap().as_bytes(),
    )?;
    Ok(Some(stats))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |flag: &str| {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let usage = "usage: relabel --in DIR --out DIR --net ID [--net-file FILE] [--nodes 5000] [--threads N] [--label-id ID] [--engine-sha SHA]";
    let (Some(input), Some(output), Some(net_id)) = (get("--in"), get("--out"), get("--net"))
    else {
        eprintln!("{usage}");
        std::process::exit(2);
    };
    let nodes: u64 = get("--nodes").map_or(5000, |n| n.parse().expect("--nodes"));
    let threads: usize = get("--threads").map_or_else(
        || std::thread::available_parallelism().map_or(1, |n| n.get()),
        |n| n.parse().expect("--threads"),
    );
    let engine_sha = get("--engine-sha").unwrap_or_else(|| "unknown".to_string());
    let label_id = get("--label-id").unwrap_or_else(|| format!("{net_id}-n{nodes}"));
    let fail = |why: String| -> ! {
        eprintln!("relabel: {why}");
        std::process::exit(2);
    };
    let (net, net_sha) = match (net_id.as_str(), get("--net-file")) {
        ("pesto", None) => (None, String::new()),
        (_, Some(file)) => {
            // A retired net: its file, verified against the recorded hash.
            let Some(retired) = retired_net(&net_id) else {
                fail(format!(
                    "--net-file is for a retired net, and {net_id} is not one (nnue::RETIRED_NETS)"
                ))
            };
            let bytes = std::fs::read(&file).unwrap_or_else(|e| fail(format!("{file}: {e}")));
            let sha = hex(&bytes);
            if sha != retired.sha256 {
                fail(format!(
                    "{file} has SHA-256 {sha}, not {net_id}'s {}",
                    retired.sha256
                ));
            }
            let net = Network::from_bytes(&bytes).unwrap_or_else(|e| fail(format!("{file}: {e}")));
            (Some(Arc::new(net)), sha)
        }
        (_, None) => match network_by_id(&net_id) {
            Some(net) => {
                let sha = NETS
                    .iter()
                    .find(|n| n.id == net_id)
                    .map(|n| hex(n.file_bytes()))
                    .unwrap_or_default();
                (Some(net), sha)
            }
            None => match retired_net(&net_id) {
                Some(retired) => fail(format!(
                    "{}; then pass it with --net-file",
                    retired_net_message(retired)
                )),
                None => fail(format!("no embedded or retired net {net_id}")),
            },
        },
    };
    rusty_chess_core::attacks::init();
    let (input, output) = (PathBuf::from(input), PathBuf::from(output));
    let todo = manifests(&input);
    let job = Job {
        input: &input,
        output: &output,
        net,
        net_id: &net_id,
        net_sha,
        nodes,
        label_id: &label_id,
        engine_sha: &engine_sha,
    };
    println!(
        "relabel: {} shards under {}, {label_id} ({nodes} nodes) on {threads} threads",
        todo.len(),
        input.display()
    );
    let (next, done, skipped) = (
        AtomicUsize::new(0),
        AtomicUsize::new(0),
        AtomicUsize::new(0),
    );
    let total = Mutex::new(RelabelStats::default());
    let failed = AtomicBool::new(false);
    let started = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
                while !failed.load(Ordering::Relaxed) {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(m) = todo.get(i) else { break };
                    match relabel_one(&job, m, &mut searcher) {
                        Ok(Some(s)) => {
                            total.lock().unwrap().add(&s);
                            let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                            if d % 100 == 0 {
                                let t = total.lock().unwrap();
                                println!(
                                    "  {d} shards, {} records, {:.0} records/s",
                                    t.records,
                                    t.records as f64 / started.elapsed().as_secs_f64()
                                );
                            }
                        }
                        Ok(None) => {
                            skipped.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            eprintln!("relabel: {e}");
                            failed.store(true, Ordering::Relaxed);
                        }
                    }
                }
            });
        }
    });
    let t = total.into_inner().unwrap();
    let mean = if t.records > 0 {
        t.sum_abs_change as f64 / t.records as f64
    } else {
        0.0
    };
    println!(
        "relabel: {} shards relabelled, {} skipped (already done); {} records, {} changed ({:.1}%), mean |change| {mean:.1} cp, max {} cp, {} mate scores clamped; {:.0}s",
        done.load(Ordering::Relaxed),
        skipped.load(Ordering::Relaxed),
        t.records,
        t.changed,
        100.0 * t.changed as f64 / t.records.max(1) as f64,
        t.max_abs_change,
        t.mates_clamped,
        started.elapsed().as_secs_f64()
    );
    if failed.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}
