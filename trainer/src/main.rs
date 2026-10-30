//! `rusty-chess-trainer`: prepare self-play data and train NNUE nets with
//! bullet.
//!
//! ```text
//! rusty-chess-trainer prepare --shards DIR --out DIR [--val-fraction 0.05] [--split-seed 1] [--shuffle-seed 1]
//! rusty-chess-trainer train --data DIR --net-id ID --wdl λ --superbatches N [--scale 400]
//!                           [--batches-per-superbatch 6104] [--batch-size 16384] [--lr 0.001]
//!                           [--lr-gamma 0.1] [--lr-step N] [--threads 4] [--out DIR]
//! ```
//!
//! `prepare` reads every shard under `--shards` (the S3 layout:
//! `.../task=*/manifests/*.json` and `.../task=*/positions/*.txt.gz`), puts
//! each *worker's* shards wholly in validation or training by a seeded hash,
//! so no game is split, parses every record with bulletformat (the parser
//! bullet's `convert --from text` uses), and writes `train.data` and
//! `val.data` in bullet's 32-byte format, shuffled with a seeded generator.
//! It writes `split.json` (seeds, the validation workers, counts, SHA-256 of
//! both outputs) and `check.fens` (validation positions for the net check).
//! The same inputs and seeds give byte-identical outputs.
//!
//! `train` trains the (768 → HIDDEN) × 2 → 1 SCReLU network the engine's
//! `crates/search/src/nnue.rs` evaluates, quantized QA = 255, QB = 64, saves
//! `net.nnue` in the engine's file format, and evaluates `check.fens` with
//! the trainer's own float network into `eval_check.json`, for comparison with
//! the engine's quantized evaluation (`nnue-eval`).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use bulletformat::{BulletFormat, ChessBoard};
use sha2::{Digest, Sha256};

/// Hidden neurons per perspective: must equal `HIDDEN` in the engine's
/// `crates/search/src/nnue.rs`.
const HIDDEN: usize = 128;
/// The engine's quantization (see `docs/nnue.md`).
const QA: i16 = 255;
const QB: i16 = 64;
/// The engine multiplies the network's output by this to get centipawns; the
/// trainer's `eval_scale` must be the same, or every evaluation is scaled.
const ENGINE_SCALE: f32 = 400.0;
/// Bytes per bulletformat record.
const RECORD: usize = std::mem::size_of::<ChessBoard>();

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("prepare") => prepare(&args[1..]),
        Some("train") => train(&args[1..]),
        _ => Err(
            "usage: rusty-chess-trainer prepare ... | train ... (see the source's header)".into(),
        ),
    };
    if let Err(e) = result {
        eprintln!("rusty-chess-trainer: {e}");
        std::process::exit(2);
    }
}

/// `--flag value` pairs.
fn flags(args: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    let mut it = args.iter();
    while let Some(f) = it.next() {
        let v = it.next().ok_or_else(|| format!("`{f}` needs a value"))?;
        out.insert(f.trim_start_matches("--").to_string(), v.clone());
    }
    Ok(out)
}

fn get<T: std::str::FromStr>(
    f: &BTreeMap<String, String>,
    key: &str,
    default: Option<T>,
) -> Result<T, String> {
    match f.get(key) {
        Some(v) => v.parse().map_err(|_| format!("--{key}: cannot read `{v}`")),
        None => default.ok_or_else(|| format!("--{key} is required")),
    }
}

/// SplitMix64: a stable, seeded mixing function for the split and the shuffle.
fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a of a string, so a worker's key hashes the same on every machine.
fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        splitmix(self.0)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// Every manifest under `dir`, sorted, with its positions file.
fn shards(dir: &Path) -> Result<Vec<(PathBuf, PathBuf, serde_json::Value)>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).map_err(|e| format!("{}: {e}", d.display()))? {
            let p = e.map_err(|e| e.to_string())?.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "json")
                && p.parent().is_some_and(|q| q.ends_with("manifests"))
            {
                let m: serde_json::Value =
                    serde_json::from_str(&std::fs::read_to_string(&p).map_err(|e| e.to_string())?)
                        .map_err(|e| format!("{}: {e}", p.display()))?;
                let name = p.file_stem().unwrap().to_string_lossy().to_string();
                let positions = p
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("positions")
                    .join(format!("{name}.txt.gz"));
                if !positions.exists() {
                    return Err(format!(
                        "{}: no positions file {}",
                        p.display(),
                        positions.display()
                    ));
                }
                out.push((p, positions, m));
            }
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

fn sha256_file(p: &Path) -> Result<String, String> {
    let mut h = Sha256::new();
    let mut f = File::open(p).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn prepare(args: &[String]) -> Result<(), String> {
    let f = flags(args)?;
    let src = PathBuf::from(get::<String>(&f, "shards", None)?);
    let out = PathBuf::from(get::<String>(&f, "out", None)?);
    let val_fraction: f64 = get(&f, "val-fraction", Some(0.05))?;
    let split_seed: u64 = get(&f, "split-seed", Some(1))?;
    let shuffle_seed: u64 = get(&f, "shuffle-seed", Some(1))?;
    let buckets: usize = get(&f, "buckets", Some(256))?;
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let list = shards(&src)?;
    if list.is_empty() {
        return Err(format!("no shards under {}", src.display()));
    }

    // Workers wholly in one split: a game never spans shards, and a worker's
    // shards share its seed.
    let worker_key = |m: &serde_json::Value| {
        format!(
            "{}/{}/{}",
            m["provenance"]["run"].as_str().unwrap_or(""),
            m["provenance"]["task"].as_str().unwrap_or(""),
            m["worker"]
        )
    };
    let is_val = |key: &str| {
        (splitmix(fnv(key) ^ splitmix(split_seed)) % 1_000_000)
            < (val_fraction * 1_000_000.0) as u64
    };

    // Pass 1: parse every record and scatter it to a random bucket file of its
    // split, so the shuffle never needs the whole dataset in memory.
    let tmp = out.join("tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut rng = Rng(splitmix(shuffle_seed));
    let mut files: Vec<Vec<BufWriter<File>>> = (0..2)
        .map(|s| {
            (0..buckets)
                .map(|b| BufWriter::new(File::create(tmp.join(format!("{s}-{b}.bin"))).unwrap()))
                .collect()
        })
        .collect();
    let mut counts = [0u64; 2];
    let mut val_workers: Vec<String> = Vec::new();
    let mut train_workers: Vec<String> = Vec::new();
    let mut check: Vec<String> = Vec::new();
    let mut manifest_positions = 0u64;
    let started = std::time::Instant::now();
    for (i, (_, positions, m)) in list.iter().enumerate() {
        let key = worker_key(m);
        let split = usize::from(is_val(&key));
        let workers = if split == 1 {
            &mut val_workers
        } else {
            &mut train_workers
        };
        if workers.last() != Some(&key) && !workers.contains(&key) {
            workers.push(key);
        }
        manifest_positions += m["positions"].as_u64().unwrap_or(0);
        let reader = BufReader::new(flate2::read::GzDecoder::new(
            File::open(positions).map_err(|e| e.to_string())?,
        ));
        for line in reader.lines() {
            let line = line.map_err(|e| format!("{}: {e}", positions.display()))?;
            let board: ChessBoard = line
                .parse()
                .map_err(|e| format!("{}: {e}: {line}", positions.display()))?;
            files[split][rng.below(buckets as u64) as usize]
                .write_all(ChessBoard::as_bytes_slice(&[board]))
                .map_err(|e| e.to_string())?;
            counts[split] += 1;
            if split == 1 && check.len() < 256 && rng.below(2000) == 0 {
                check.push(line.split(" | ").next().unwrap().to_string());
            }
        }
        if (i + 1) % 500 == 0 {
            eprintln!(
                "prepare: {} of {} shards, {} records, {:.0}s",
                i + 1,
                list.len(),
                counts[0] + counts[1],
                started.elapsed().as_secs_f64()
            );
        }
    }
    drop(files);
    if counts[0] + counts[1] != manifest_positions {
        return Err(format!(
            "parsed {} records but the manifests say {manifest_positions}",
            counts[0] + counts[1]
        ));
    }

    // Pass 2: shuffle each bucket in memory and append it to its output.
    for (split, name) in [(0usize, "train.data"), (1, "val.data")] {
        let mut w = BufWriter::new(File::create(out.join(name)).map_err(|e| e.to_string())?);
        for b in 0..buckets {
            let p = tmp.join(format!("{split}-{b}.bin"));
            let mut bytes = Vec::new();
            File::open(&p)
                .and_then(|mut f| f.read_to_end(&mut bytes))
                .map_err(|e| e.to_string())?;
            let n = bytes.len() / RECORD;
            for i in (1..n).rev() {
                let j = rng.below(i as u64 + 1) as usize;
                if i != j {
                    for k in 0..RECORD {
                        bytes.swap(i * RECORD + k, j * RECORD + k);
                    }
                }
            }
            w.write_all(&bytes).map_err(|e| e.to_string())?;
            std::fs::remove_file(&p).ok();
        }
        w.flush().map_err(|e| e.to_string())?;
    }
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::write(out.join("check.fens"), check.join("\n") + "\n").map_err(|e| e.to_string())?;

    let summary = serde_json::json!({
        "schema": "rusty-chess-prepared/1",
        "source": src.display().to_string(),
        "shards": list.len(),
        "records": {"train": counts[0], "val": counts[1]},
        "workers": {"train": train_workers.len(), "val": val_workers.len()},
        "val_workers": val_workers,
        "val_fraction": val_fraction,
        "split_seed": split_seed,
        "split": "a worker (run/task/worker) is validation when splitmix(fnv(key) ^ splitmix(split_seed)) % 1e6 < val_fraction × 1e6",
        "shuffle_seed": shuffle_seed,
        "shuffle": format!("records scattered to {buckets} buckets by a seeded generator, each bucket Fisher-Yates shuffled"),
        "format": format!("bulletformat 1.8.0 ChessBoard, {RECORD} bytes per record"),
        "sha256": {"train.data": sha256_file(&out.join("train.data"))?, "val.data": sha256_file(&out.join("val.data"))?},
        "check_fens": check.len(),
        "seconds": started.elapsed().as_secs(),
    });
    std::fs::write(
        out.join("split.json"),
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::to_string_pretty(&summary)
            .unwrap()
            .lines()
            .filter(|l| !l.contains("\"val_workers\"") && !l.trim_start().starts_with("\"run="))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}

fn train(args: &[String]) -> Result<(), String> {
    use bullet_lib::{
        game::inputs::Chess768,
        nn::optimiser::AdamW,
        trainer::{
            save::SavedFormat,
            schedule::{TrainingSchedule, TrainingSteps, lr, wdl},
            settings::{LocalSettings, TestDataset},
        },
        value::{ValueTrainerBuilder, loader},
    };
    let f = flags(args)?;
    let data = PathBuf::from(get::<String>(&f, "data", None)?);
    let net_id: String = get(&f, "net-id", None)?;
    let wdl_value: f32 = get(&f, "wdl", None)?;
    let superbatches: usize = get(&f, "superbatches", None)?;
    let scale: f32 = get(&f, "scale", Some(ENGINE_SCALE))?;
    let batches: usize = get(&f, "batches-per-superbatch", Some(6104))?;
    let batch_size: usize = get(&f, "batch-size", Some(16_384))?;
    let lr_start: f32 = get(&f, "lr", Some(0.001))?;
    let lr_gamma: f32 = get(&f, "lr-gamma", Some(0.1))?;
    let lr_step: usize = get(&f, "lr-step", Some((superbatches * 9 / 20).max(1)))?;
    let threads: usize = get(&f, "threads", Some(4))?;
    let out: String = get(&f, "out", Some("checkpoints".to_string()))?;
    if (scale - ENGINE_SCALE).abs() > f32::EPSILON {
        return Err(format!(
            "--scale {scale}: the engine multiplies the output by {ENGINE_SCALE}; train at that scale or change SCALE in the engine too"
        ));
    }
    if !(0.0..=1.0).contains(&wdl_value) {
        return Err("--wdl must be in 0..=1".into());
    }
    let train_file = data.join("train.data");
    let val_file = data.join("val.data");
    let train_str = train_file.to_string_lossy().to_string();
    let val_str = val_file.to_string_lossy().to_string();

    let mut trainer = ValueTrainerBuilder::default()
        .dual_perspective()
        .optimiser(AdamW)
        .inputs(Chess768)
        .save_format(&[
            SavedFormat::id("l0w").round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            SavedFormat::id("l1w").round().quantise::<i16>(QB),
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ])
        .loss_fn(|output, target| output.sigmoid().squared_error(target))
        .build(|builder, stm, ntm| {
            let l0 = builder.new_affine("l0", 768, HIDDEN);
            let l1 = builder.new_affine("l1", 2 * HIDDEN, 1);
            let us = l0.forward(stm).screlu();
            let them = l0.forward(ntm).screlu();
            l1.forward(us.concat(them))
        });

    let schedule = TrainingSchedule {
        net_id: net_id.clone(),
        eval_scale: scale,
        steps: TrainingSteps {
            batch_size,
            batches_per_superbatch: batches,
            start_superbatch: 1,
            end_superbatch: superbatches,
        },
        wdl_scheduler: wdl::ConstantWDL { value: wdl_value },
        lr_scheduler: lr::StepLR {
            start: lr_start,
            gamma: lr_gamma,
            step: lr_step,
        },
        save_rate: superbatches.max(1),
    };
    let settings = LocalSettings {
        threads,
        test_set: val_file.exists().then(|| TestDataset::at(val_str.as_str())),
        output_directory: out.as_str(),
        batch_queue_size: 64,
    };
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let loader = loader::DirectSequentialDataLoader::new(&[train_str.as_str()]);
    trainer.run(&schedule, &settings, &loader);

    // The quantised net, in the engine's format.
    let ckpt = PathBuf::from(&out).join(format!("{net_id}-{superbatches}"));
    let net = std::fs::read(ckpt.join("quantised.bin"))
        .map_err(|e| format!("{}: {e}", ckpt.display()))?;
    std::fs::write(PathBuf::from(&out).join("net.nnue"), &net).map_err(|e| e.to_string())?;

    // The trainer's own (float) evaluation of the check positions, in
    // centipawns for the side to move, for comparison with the engine's.
    let fens = std::fs::read_to_string(data.join("check.fens")).unwrap_or_default();
    let mut checks = Vec::new();
    for fen in fens.lines().filter(|l| !l.is_empty()) {
        // bulletformat's parser wants a full record: a score and result are
        // placeholders here.
        let cp = trainer.eval(&format!("{fen} | 0 | 0.5")) * scale;
        checks.push(serde_json::json!({"fen": fen, "trainer_cp": cp}));
    }
    std::fs::write(
        PathBuf::from(&out).join("eval_check.json"),
        serde_json::to_string_pretty(&checks).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "trained {net_id}: {} bytes, {} check positions evaluated",
        net.len(),
        checks.len()
    );
    Ok(())
}
