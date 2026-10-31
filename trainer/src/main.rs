//! `rusty-chess-trainer`: prepare self-play data and train NNUE nets with
//! bullet.
//!
//! ```text
//! rusty-chess-trainer prepare --shards DIR --out DIR [--val-fraction 0.05] [--split-seed 1] [--shuffle-seed 1] [--threads N]
//! rusty-chess-trainer train --data DIR --net-id ID --wdl λ --superbatches N [--hidden 128] [--scale 400]
//!                           [--batches-per-superbatch 6104] [--batch-size 16384] [--lr 0.001]
//!                           [--lr-gamma 0.1] [--lr-step N] [--threads 4] [--out DIR]
//!                           [--checkpoint-every 4] [--val-positions 4000000]
//! rusty-chess-trainer validate --net FILE --data DIR --wdl λ [--positions 4000000] [--threads 4]
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
//! `train` trains the (768 → H) × 2 → 1 SCReLU network (H = `--hidden`) the engine's
//! `crates/search/src/nnue.rs` evaluates, quantized QA = 255, QB = 64, saves
//! `net.nnue` in the engine's file format, and evaluates `check.fens` with
//! the trainer's own float network into `eval_check.json`, for comparison with
//! the engine's quantized evaluation (`nnue-eval`).
//!
//! bullet (at the pinned commit) does not score a validation set, so `train`
//! does: it saves a checkpoint every `--checkpoint-every` superbatches and,
//! after training, scores each one's float weights on the first
//! `--val-positions` records of `val.data` and as many of `train.data` (both
//! files are shuffled, so these are random samples), with the loss bullet
//! trains on. The curve goes to `val_losses.txt` and `validation.json`.
//! `validate` scores any net file (bullet's `raw.bin`, or a quantised
//! `net.nnue`) the same way after the fact.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use bulletformat::{BulletFormat, ChessBoard};
use sha2::{Digest, Sha256};

/// Hidden neurons per perspective unless `--hidden` says otherwise. The
/// engine must read the same width (`nnue::HIDDEN` there).
const DEFAULT_HIDDEN: usize = 128;
/// The widest net the scorer handles (its accumulators live on the stack).
const MAX_HIDDEN: usize = 1024;

/// The number of weights and biases in a (768 → h) × 2 → 1 net.
const fn params(h: usize) -> usize {
    768 * h + h + 2 * h + 1
}
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
        Some("validate") => validate(&args[1..]),
        _ => Err(
            "usage: rusty-chess-trainer prepare ... | train ... | validate ... (see the source's header)"
                .into(),
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

/// A writer that hashes what it writes, so a prepared file's SHA-256 needs no
/// second read of it.
struct HashingWriter<W: Write> {
    inner: W,
    hash: Sha256,
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.hash.update(&buf[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// One shard's records, parsed, with its decompressed text and each held-out
/// record's line span in it (the check positions are sampled from those).
struct Parsed {
    boards: Vec<ChessBoard>,
    text: String,
    lines: Vec<(u32, u32)>,
}

impl Parsed {
    /// The FEN of held-out record `r`.
    fn fen(&self, r: usize) -> &str {
        let (a, b) = self.lines[r];
        let line = &self.text[a as usize..b as usize];
        line.split(" | ").next().unwrap()
    }
}

/// Decompress and parse one shard's positions file.
fn parse_shard(positions: &Path, held_out: bool) -> Result<Parsed, String> {
    let mut text = String::new();
    flate2::read::GzDecoder::new(File::open(positions).map_err(|e| e.to_string())?)
        .read_to_string(&mut text)
        .map_err(|e| format!("{}: {e}", positions.display()))?;
    let mut boards = Vec::with_capacity(25_000);
    let mut lines = Vec::new();
    let mut at = 0usize;
    for line in text.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            continue;
        }
        let board: ChessBoard = line
            .parse()
            .map_err(|e| format!("{}: {e}: {line}", positions.display()))?;
        boards.push(board);
        if held_out {
            lines.push((start as u32, (start + line.len()) as u32));
        }
    }
    Ok(Parsed {
        boards,
        text,
        lines,
    })
}

fn prepare(args: &[String]) -> Result<(), String> {
    let f = flags(args)?;
    let src = PathBuf::from(get::<String>(&f, "shards", None)?);
    let out = PathBuf::from(get::<String>(&f, "out", None)?);
    let val_fraction: f64 = get(&f, "val-fraction", Some(0.05))?;
    let split_seed: u64 = get(&f, "split-seed", Some(1))?;
    let shuffle_seed: u64 = get(&f, "shuffle-seed", Some(1))?;
    let buckets: usize = get(&f, "buckets", Some(256))?;
    let threads: usize = get(
        &f,
        "threads",
        Some(std::thread::available_parallelism().map_or(1, |n| n.get())),
    )?;
    let threads = threads.max(1);
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
    // split, so the shuffle never needs the whole dataset in memory. Parsing
    // (gunzip and text) runs on `threads` threads, a window of shards at a
    // time; the scatter then takes the parsed shards in their original order,
    // drawing from one generator exactly as a single thread would, so the
    // output is the same byte for byte whatever the thread count.
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
    let splits: Vec<usize> = list
        .iter()
        .map(|(_, _, m)| usize::from(is_val(&worker_key(m))))
        .collect();
    let window = threads * 8;
    for (w, chunk) in list.chunks(window).enumerate() {
        let base = w * window;
        let parsed: Vec<Result<Parsed, String>> = std::thread::scope(|scope| {
            let per = chunk.len().div_ceil(threads);
            let handles: Vec<_> = chunk
                .chunks(per)
                .enumerate()
                .map(|(t, part)| {
                    let splits = &splits;
                    scope.spawn(move || {
                        part.iter()
                            .enumerate()
                            .map(|(k, (_, positions, _))| {
                                parse_shard(positions, splits[base + t * per + k] == 1)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles
                .into_iter()
                .flat_map(|h| h.join().expect("a parsing thread panicked"))
                .collect()
        });
        for (k, ((_, _, m), shard)) in chunk.iter().zip(parsed).enumerate() {
            let i = base + k;
            let shard = shard?;
            let key = worker_key(m);
            let split = splits[i];
            let workers = if split == 1 {
                &mut val_workers
            } else {
                &mut train_workers
            };
            if workers.last() != Some(&key) && !workers.contains(&key) {
                workers.push(key);
            }
            manifest_positions += m["positions"].as_u64().unwrap_or(0);
            for (r, board) in shard.boards.iter().enumerate() {
                files[split][rng.below(buckets as u64) as usize]
                    .write_all(ChessBoard::as_bytes_slice(std::slice::from_ref(board)))
                    .map_err(|e| e.to_string())?;
                counts[split] += 1;
                if split == 1 && check.len() < 256 && rng.below(2000) == 0 {
                    check.push(shard.fen(r).to_string());
                }
            }
            if (i + 1).is_multiple_of(500) {
                eprintln!(
                    "prepare: {} of {} shards, {} records, {:.0}s",
                    i + 1,
                    list.len(),
                    counts[0] + counts[1],
                    started.elapsed().as_secs_f64()
                );
            }
        }
    }
    drop(files);
    eprintln!(
        "prepare: pass 1 (parse and scatter, {threads} threads) {:.1}s",
        started.elapsed().as_secs_f64()
    );
    if counts[0] + counts[1] != manifest_positions {
        return Err(format!(
            "parsed {} records but the manifests say {manifest_positions}",
            counts[0] + counts[1]
        ));
    }

    // Pass 2: shuffle each bucket in memory and append it to its output,
    // hashing the output as it is written.
    let mut digests: Vec<String> = Vec::new();
    for (split, name) in [(0usize, "train.data"), (1, "val.data")] {
        let mut w = HashingWriter {
            inner: BufWriter::new(File::create(out.join(name)).map_err(|e| e.to_string())?),
            hash: Sha256::new(),
        };
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
                    // j < i: swap the two whole records.
                    let (low, high) = bytes.split_at_mut(i * RECORD);
                    low[j * RECORD..(j + 1) * RECORD].swap_with_slice(&mut high[..RECORD]);
                }
            }
            w.write_all(&bytes).map_err(|e| e.to_string())?;
            std::fs::remove_file(&p).ok();
        }
        w.flush().map_err(|e| e.to_string())?;
        digests.push(
            w.hash
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
        );
    }
    eprintln!(
        "prepare: pass 2 (shuffle and write) done at {:.1}s",
        started.elapsed().as_secs_f64()
    );
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
        "sha256": {"train.data": digests[0], "val.data": digests[1]},
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
    let checkpoint_every: usize = get(&f, "checkpoint-every", Some(4))?;
    let val_positions: usize = get(&f, "val-positions", Some(4_000_000))?;
    let hidden: usize = get(&f, "hidden", Some(DEFAULT_HIDDEN))?;
    if hidden == 0 || !hidden.is_multiple_of(16) || hidden > MAX_HIDDEN {
        return Err(format!(
            "--hidden {hidden}: a multiple of 16 up to {MAX_HIDDEN} (the engine's SIMD works in 16s)"
        ));
    }
    if checkpoint_every == 0 {
        return Err("--checkpoint-every must be at least 1".into());
    }
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
            let l0 = builder.new_affine("l0", 768, hidden);
            let l1 = builder.new_affine("l1", 2 * hidden, 1);
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
        // Every checkpoint is scored on held-out data after training.
        save_rate: checkpoint_every,
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

    // The scorer must be bullet's network: its float forward pass on the
    // final weights against bullet's own evaluation of the check positions.
    let final_net = FloatNet::from_file(&ckpt.join("raw.bin"))?;
    let mut agree = 0f32;
    for (fen, c) in fens.lines().filter(|l| !l.is_empty()).zip(&checks) {
        let board = record(fen)?;
        let ours = final_net.forward(&board) * scale;
        agree = agree.max((ours - c["trainer_cp"].as_f64().unwrap() as f32).abs());
    }
    println!(
        "validation scorer against bullet's evaluation: max |diff| {agree:.4} cp over {} positions",
        checks.len()
    );
    if agree > 0.5 {
        return Err(format!(
            "the validation scorer disagrees with bullet's evaluation by {agree} cp; its losses would be wrong"
        ));
    }

    // Held-out loss at every checkpoint.
    let val = read_records(&val_file, val_positions)?;
    let train_sample = read_records(&train_file, val.len())?;
    let mut points = Vec::new();
    let mut sb = checkpoint_every.min(superbatches);
    let started = std::time::Instant::now();
    println!("superbatch  train loss (sample)  val loss");
    loop {
        let dir = PathBuf::from(&out).join(format!("{net_id}-{sb}"));
        let net = FloatNet::from_file(&dir.join("raw.bin"))?;
        let t = net.loss(&train_sample, wdl_value, scale, threads);
        let v = net.loss(&val, wdl_value, scale, threads);
        println!("{sb:>10}  {t:>19.6}  {v:.6}");
        points.push((sb, t, v));
        if sb == superbatches {
            break;
        }
        sb = (sb + checkpoint_every).min(superbatches);
    }
    let best = points
        .iter()
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|p| p.0)
        .unwrap_or(superbatches);
    let curve: String = points
        .iter()
        .map(|(s, t, v)| format!("{s},{t},{v}\n"))
        .collect();
    std::fs::write(
        PathBuf::from(&out).join("val_losses.txt"),
        format!("superbatch,train_loss_sample,val_loss\n{curve}"),
    )
    .map_err(|e| e.to_string())?;
    let validation = serde_json::json!({
        "loss": "mean over positions of (sigmoid(output) - (λ·result + (1 - λ)·sigmoid(score / 400)))², the float network bullet trains (not the quantised one)",
        "val_positions": val.len(),
        "train_sample_positions": train_sample.len(),
        "checkpoint_every": checkpoint_every,
        "scorer_max_diff_cp": agree,
        "points": points.iter().map(|(s, t, v)| serde_json::json!({"superbatch": s, "train_loss_sample": t, "val_loss": v})).collect::<Vec<_>>(),
        "best_superbatch": best,
        "seconds": started.elapsed().as_secs(),
    });
    std::fs::write(
        PathBuf::from(&out).join("validation.json"),
        serde_json::to_string_pretty(&validation).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "validation: {} held-out positions, best at superbatch {best} of {superbatches}, scored in {:.0}s",
        val.len(),
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

/// `validate`: the held-out loss of any net file, e.g. a net trained before
/// `train` scored its checkpoints.
fn validate(args: &[String]) -> Result<(), String> {
    let f = flags(args)?;
    let net = FloatNet::from_file(Path::new(&get::<String>(&f, "net", None)?))?;
    let data = PathBuf::from(get::<String>(&f, "data", None)?);
    let wdl_value: f32 = get(&f, "wdl", None)?;
    let positions: usize = get(&f, "positions", Some(4_000_000))?;
    let threads: usize = get(&f, "threads", Some(4))?;
    let started = std::time::Instant::now();
    let val = read_records(&data.join("val.data"), positions)?;
    let v = net.loss(&val, wdl_value, ENGINE_SCALE, threads);
    let mut out = serde_json::json!({"wdl": wdl_value, "val_positions": val.len(), "val_loss": v});
    let train_file = data.join("train.data");
    if train_file.exists() {
        let sample = read_records(&train_file, val.len())?;
        out["train_sample_positions"] = sample.len().into();
        out["train_loss_sample"] = net.loss(&sample, wdl_value, ENGINE_SCALE, threads).into();
    }
    out["weights"] = net.source.into();
    out["seconds"] = started.elapsed().as_secs_f64().into();
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    Ok(())
}

/// One bulletformat record from a FEN (the score and result are placeholders).
fn record(fen: &str) -> Result<ChessBoard, String> {
    format!("{fen} | 0 | 0.5")
        .parse()
        .map_err(|e| format!("{fen}: {e:?}"))
}

/// The first `n` records of a prepared file (all of them if it has fewer).
fn read_records(path: &Path, n: usize) -> Result<Vec<ChessBoard>, String> {
    let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take((n * RECORD) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(bytes
        .as_chunks::<RECORD>()
        .0
        .iter()
        // SAFETY: ChessBoard is `repr(C)` and 32 bytes, with no invalid bit
        // patterns; the read is unaligned-safe.
        .map(|c| unsafe { std::ptr::read_unaligned(c.as_ptr().cast::<ChessBoard>()) })
        .collect())
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// The (768 → h) × 2 → 1 SCReLU network in floats, as bullet trains it:
/// output in units of `eval_scale` centipawns. The width `h` comes from the
/// file's size.
struct FloatNet {
    hidden: usize,
    /// Feature-major: feature `i`'s weights are `l0w[i * hidden..][..hidden]`.
    l0w: Vec<f32>,
    l0b: Vec<f32>,
    /// The side to move's half first.
    l1w: Vec<f32>,
    l1b: f32,
    source: &'static str,
}

impl FloatNet {
    /// bullet's `raw.bin` (f32 weights in the saved format's order) or a
    /// quantised net (`quantised.bin`, `net.nnue`: i16, padded to 64 bytes),
    /// told apart by size, at any width the scorer handles.
    fn from_file(path: &Path) -> Result<FloatNet, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let widths = (16..=MAX_HIDDEN).step_by(16);
        if let Some(h) = widths.clone().find(|&h| bytes.len() == 4 * params(h)) {
            let w: Vec<f32> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&c| f32::from_le_bytes(c))
                .collect();
            Ok(FloatNet::from_params(&w, h, "float (raw.bin)"))
        } else if let Some(h) = widths
            .clone()
            .find(|&h| bytes.len() >= 2 * params(h) && bytes.len() - 2 * params(h) < 64)
        {
            let n = params(h);
            let q: Vec<f32> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .take(n)
                .map(|&c| f32::from(i16::from_le_bytes(c)))
                .collect();
            let (qa, qb) = (f32::from(QA), f32::from(QB));
            let l1 = 768 * h + h;
            let w: Vec<f32> = q
                .iter()
                .enumerate()
                .map(|(i, &x)| match i {
                    _ if i < l1 => x / qa,
                    _ if i < n - 1 => x / qb,
                    _ => x / (qa * qb),
                })
                .collect();
            Ok(FloatNet::from_params(
                &w,
                h,
                "quantised weights, dequantised to floats",
            ))
        } else {
            Err(format!(
                "{}: {} bytes is not a float or quantised (768 → h) × 2 → 1 net for any h in 16..={MAX_HIDDEN} (h = 128: {} or {} bytes)",
                path.display(),
                bytes.len(),
                4 * params(128),
                2 * params(128)
            ))
        }
    }

    fn from_params(w: &[f32], hidden: usize, source: &'static str) -> FloatNet {
        let (l0w, rest) = w.split_at(768 * hidden);
        let (l0b, rest) = rest.split_at(hidden);
        let (l1w, l1b) = rest.split_at(2 * hidden);
        FloatNet {
            hidden,
            l0w: l0w.to_vec(),
            l0b: l0b.to_vec(),
            l1w: l1w.to_vec(),
            l1b: l1b[0],
            source,
        }
    }

    /// The output for the side to move, with bullet's `Chess768` features:
    /// records are stored from the side to move's view, so its pieces have
    /// colour bit 0.
    fn forward(&self, b: &ChessBoard) -> f32 {
        let h = self.hidden;
        let mut us_buf = [0f32; MAX_HIDDEN];
        let mut them_buf = [0f32; MAX_HIDDEN];
        let (us, them) = (&mut us_buf[..h], &mut them_buf[..h]);
        us.copy_from_slice(&self.l0b);
        them.copy_from_slice(&self.l0b);
        for (piece, square) in *b {
            let c = usize::from(piece & 8 > 0);
            let pc = 64 * usize::from(piece & 7);
            let sq = usize::from(square);
            let stm = [0, 384][c] + pc + sq;
            let ntm = [384, 0][c] + pc + (sq ^ 56);
            let (ws, wn) = (&self.l0w[stm * h..][..h], &self.l0w[ntm * h..][..h]);
            for i in 0..h {
                us[i] += ws[i];
                them[i] += wn[i];
            }
        }
        let screlu = |x: f32| {
            let c = x.clamp(0.0, 1.0);
            c * c
        };
        let mut out = self.l1b;
        for i in 0..h {
            out += screlu(us[i]) * self.l1w[i] + screlu(them[i]) * self.l1w[h + i];
        }
        out
    }

    /// Mean of bullet's loss over `records`, on `threads` threads.
    fn loss(&self, records: &[ChessBoard], wdl: f32, scale: f32, threads: usize) -> f64 {
        if records.is_empty() {
            return f64::NAN;
        }
        let chunk = records.len().div_ceil(threads.max(1));
        let total: f64 = std::thread::scope(|s| {
            records
                .chunks(chunk)
                .map(|part| {
                    s.spawn(move || {
                        part.iter()
                            .map(|b| {
                                let result = f32::from(b.result) / 2.0;
                                let target = wdl * result
                                    + (1.0 - wdl) * sigmoid(f32::from(b.score) / scale);
                                let e = sigmoid(self.forward(b)) - target;
                                f64::from(e * e)
                            })
                            .sum::<f64>()
                    })
                })
                .collect::<Vec<_>>()
                .into_iter()
                .map(|h| h.join().unwrap())
                .sum()
        });
        total / records.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net_with(
        hidden: usize,
        l0: impl Fn(usize, usize) -> f32,
        l1: impl Fn(usize) -> f32,
        l1b: f32,
    ) -> FloatNet {
        let mut w = Vec::with_capacity(params(hidden));
        for f in 0..768 {
            for h in 0..hidden {
                w.push(l0(f, h));
            }
        }
        w.extend((0..hidden).map(|_| 0.0));
        w.extend((0..2 * hidden).map(l1));
        w.push(l1b);
        FloatNet::from_params(&w, hidden, "test")
    }

    #[test]
    fn forward_follows_chess768_and_screlu() {
        for hidden in [128, 256] {
            forward_at(hidden);
        }
    }

    fn forward_at(hidden: usize) {
        // A white pawn on e2 is feature 12 for the side to move (white) and
        // 384 + (12 ^ 56) = 436 for the other side; weight only the first.
        let net = net_with(
            hidden,
            |f, h| match (f, h) {
                (12, 2) => 0.7,
                (12, 3) => 3.0, // SCReLU clamps it to 1
                _ => 0.0,
            },
            |i| match i {
                2 => 1.0,
                3 => 0.5,
                i if i >= hidden => 10.0, // the other side's half must stay 0
                _ => 0.0,
            },
            0.25,
        );
        let want = 0.25 + 0.7 * 0.7 + 0.5;
        let white = record("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1").unwrap();
        assert!(
            (net.forward(&white) - want).abs() < 1e-6,
            "{}",
            net.forward(&white)
        );
        // The mirror with black to move is the same position for the side to
        // move: records are stored from its view.
        let black = record("4k3/4p3/8/8/8/8/8/4K3 b - - 0 1").unwrap();
        assert!(
            (net.forward(&black) - want).abs() < 1e-6,
            "{}",
            net.forward(&black)
        );
    }

    #[test]
    fn loss_is_bullets_target() {
        // A zero net outputs 0: sigmoid 0.5 against the blended target.
        let net = net_with(DEFAULT_HIDDEN, |_, _| 0.0, |_| 0.0, 0.0);
        let mut b = record("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        b.score = 400;
        b.result = 2; // a win for the side to move
        let target = 0.5 * 1.0 + 0.5 * sigmoid(1.0);
        let want = f64::from((0.5 - target) * (0.5 - target));
        let got = net.loss(&[b, b, b], 0.5, 400.0, 2);
        assert!((got - want).abs() < 1e-9, "{got} vs {want}");
    }

    #[test]
    fn quantised_file_dequantises_at_any_width() {
        let dir = std::env::temp_dir().join(format!("rc-trainer-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for hidden in [128, 256] {
            let n = params(hidden);
            let mut q = vec![0i16; n];
            q[0] = QA; // l0w → 1.0
            q[768 * hidden + hidden] = QB; // l1w[0] → 1.0
            q[n - 1] = QA * QB / 2; // l1b → 0.5
            let mut bytes: Vec<u8> = q.iter().flat_map(|w| w.to_le_bytes()).collect();
            bytes.resize(bytes.len().next_multiple_of(64), 0);
            let p = dir.join(format!("net{hidden}.nnue"));
            std::fs::write(&p, &bytes).unwrap();
            let net = FloatNet::from_file(&p).unwrap();
            assert_eq!(net.hidden, hidden);
            assert_eq!(net.l0w[0], 1.0);
            assert_eq!(net.l1w[0], 1.0);
            assert_eq!(net.l1b, 0.5);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
