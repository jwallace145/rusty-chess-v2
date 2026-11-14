//! `braveheart-trainer`: prepare self-play data and train NNUE nets with
//! bullet.
//!
//! ```text
//! braveheart-trainer prepare --shards DIR --out DIR [--val-fraction 0.05] [--split-seed 1] [--shuffle-seed 1] [--threads N]
//! braveheart-trainer train --data DIR --net-id ID --wdl λ --superbatches N [--hidden 128] [--output-buckets 1] [--king-buckets 1] [--scale 400]
//!                           [--batches-per-superbatch 6104] [--batch-size 16384] [--lr 0.001]
//!                           [--lr-gamma 0.1] [--lr-step N] [--threads 4] [--out DIR]
//!                           [--checkpoint-every 4] [--val-positions 4000000]
//! braveheart-trainer validate --net FILE --data DIR --wdl λ [--positions 4000000] [--threads 4]
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
//! `train` trains the (768 × K → H) × 2 → B SCReLU network (H = `--hidden`, B =
//! `--output-buckets`, one output chosen per position by its piece count, K =
//! `--king-buckets`, each side's inputs indexed by its own king's bucket and
//! mirrored with the king on files e–h; `docs/king-buckets.md`) the engine's
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
/// engine must read that width (`nnue::WIDTHS` there).
const DEFAULT_HIDDEN: usize = 128;
/// The widest net the scorer handles (its accumulators live on the stack).
const MAX_HIDDEN: usize = 1024;

/// The output bucket counts the trainer builds: one output layer, or one per
/// band of piece counts. The engine must read the count (`nnue::OUTPUT_BUCKETS`
/// there).
const OUTPUT_BUCKETS: [usize; 2] = [1, 8];

/// The king bucket counts the trainer builds: one view, or the engine's four
/// (`nnue::KING_BUCKETS` there).
const KING_BUCKETS: [usize; 2] = [1, 4];

/// The engine's `nnue::KING_BUCKET_LAYOUT`: the king bucket of each square a
/// mirrored king can stand on, files a–d of ranks 1 to 8 from its own side. A
/// test checks the two tables are the same.
#[rustfmt::skip]
const KING_BUCKET_LAYOUT: [usize; 32] = [
    0, 0, 1, 1,
    2, 2, 2, 2,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
];

/// The number of weights and biases in a (768 × k → h) × 2 → b net.
const fn params(h: usize, b: usize, k: usize) -> usize {
    768 * k * h + h + b * (2 * h + 1)
}

/// A perspective's view: its king bucket's feature offset and the XOR that
/// mirrors a square's file, from its own king square `ksq` seen from its own
/// side. bullet's `ChessBucketsMirrored`; the engine's `nnue::king_view`.
fn king_view(king_buckets: usize, ksq: usize) -> (usize, usize) {
    if king_buckets == 1 {
        return (0, 0);
    }
    let file = ksq % 8;
    let (flip, half) = if file > 3 { (7, 7 - file) } else { (0, file) };
    (768 * KING_BUCKET_LAYOUT[(ksq / 8) * 4 + half], flip)
}

/// The output bucket of a position with `pieces` pieces in a net of `buckets`:
/// bullet's `MaterialCount<buckets>`, `(pieces − 2) / (32 / buckets)`, and the
/// engine's `nnue::output_bucket`.
fn output_bucket(pieces: usize, buckets: usize) -> usize {
    (pieces.saturating_sub(2) / 32usize.div_ceil(buckets)).min(buckets - 1)
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
            "usage: braveheart-trainer prepare ... | train ... | validate ... (see the source's header)"
                .into(),
        ),
    };
    if let Err(e) = result {
        eprintln!("braveheart-trainer: {e}");
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
        game::{
            inputs::{Chess768, ChessBucketsMirrored},
            outputs::MaterialCount,
        },
        nn::{
            InitSettings, Shape,
            optimiser::{AdamW, AdamWParams},
        },
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
    let buckets: usize = get(&f, "output-buckets", Some(1))?;
    if !OUTPUT_BUCKETS.contains(&buckets) {
        return Err(format!(
            "--output-buckets {buckets}: one of {OUTPUT_BUCKETS:?}"
        ));
    }
    let king_buckets: usize = get(&f, "king-buckets", Some(1))?;
    if !KING_BUCKETS.contains(&king_buckets) {
        return Err(format!(
            "--king-buckets {king_buckets}: one of {KING_BUCKETS:?}"
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
    let fens = std::fs::read_to_string(data.join("check.fens")).unwrap_or_default();

    // The engine's file format. bullet keeps a layer's weights column-major,
    // so a bucketed output layer's raw weights interleave the buckets (input
    // 0 of every bucket, then input 1, ...); transposed, each bucket's 2H
    // weights are contiguous, the engine's `[B][2H]`. At one bucket the two
    // orders are the same, and the file is byte for byte what it was.
    //
    // With king buckets, the factoriser `l0f` (768 × H, shared by every king
    // bucket) is added to each bucket's weights in the saved net. bullet's
    // raw.bin skips save transforms, so `l0f` is also listed last: raw.bin
    // then ends with it (the float scorer adds it back) and so does
    // quantised.bin, which is cut to the engine's size below.
    let formats = || {
        let l1w = SavedFormat::id("l1w").round().quantise::<i16>(QB);
        let l0w = SavedFormat::id("l0w");
        let l0w = if king_buckets > 1 {
            l0w.transform(move |store, weights| {
                let factoriser = store.get("l0f").values.f32().repeat(king_buckets);
                weights
                    .into_iter()
                    .zip(factoriser)
                    .map(|(a, b)| a + b)
                    .collect()
            })
        } else {
            l0w
        };
        let mut formats = vec![
            l0w.round().quantise::<i16>(QA),
            SavedFormat::id("l0b").round().quantise::<i16>(QA),
            if buckets > 1 { l1w.transpose() } else { l1w },
            SavedFormat::id("l1b").round().quantise::<i16>(QA * QB),
        ];
        if king_buckets > 1 {
            formats.push(SavedFormat::id("l0f").round().quantise::<i16>(QA));
        }
        formats
    };
    // Train, then the trainer's own (float) evaluation of the check
    // positions, in centipawns for the side to move, for comparison with the
    // engine's. bulletformat's parser wants a full record: a score and result
    // are placeholders here. The networks are different types, hence the
    // macro. With king buckets, `l0w` and `l0f` are each clipped to ±0.99, so
    // their sum stays within the ±1.98 the quantisation assumes.
    macro_rules! run_and_evaluate {
        ($trainer:expr) => {{
            let mut trainer = $trainer;
            if king_buckets > 1 {
                let clip = AdamWParams {
                    max_weight: 0.99,
                    min_weight: -0.99,
                    ..Default::default()
                };
                trainer.optimiser.set_params_for_weight("l0w", clip);
                trainer.optimiser.set_params_for_weight("l0f", clip);
            }
            trainer.run(&schedule, &settings, &loader);
            fens.lines()
                .filter(|l| !l.is_empty())
                .map(|fen| trainer.eval(&format!("{fen} | 0 | 0.5")) * scale)
                .collect::<Vec<f32>>()
        }};
    }
    let layout = ChessBucketsMirrored::new(KING_BUCKET_LAYOUT);
    let trainer_cps = match (buckets, king_buckets) {
        (1, 1) => run_and_evaluate!(
            ValueTrainerBuilder::default()
                .dual_perspective()
                .optimiser(AdamW)
                .inputs(Chess768)
                .save_format(&formats())
                .loss_fn(|output, target| output.sigmoid().squared_error(target))
                .build(|builder, stm, ntm| {
                    let l0 = builder.new_affine("l0", 768, hidden);
                    let l1 = builder.new_affine("l1", 2 * hidden, 1);
                    let us = l0.forward(stm).screlu();
                    let them = l0.forward(ntm).screlu();
                    l1.forward(us.concat(them))
                })
        ),
        (_, 1) => run_and_evaluate!(
            ValueTrainerBuilder::default()
                .dual_perspective()
                .optimiser(AdamW)
                .inputs(Chess768)
                .output_buckets(MaterialCount::<8>)
                .save_format(&formats())
                .loss_fn(|output, target| output.sigmoid().squared_error(target))
                .build(|builder, stm, ntm, output_buckets| {
                    let l0 = builder.new_affine("l0", 768, hidden);
                    let l1 = builder.new_affine("l1", 2 * hidden, 8);
                    let us = l0.forward(stm).screlu();
                    let them = l0.forward(ntm).screlu();
                    l1.forward(us.concat(them)).select(output_buckets)
                })
        ),
        (1, _) => run_and_evaluate!(
            ValueTrainerBuilder::default()
                .dual_perspective()
                .optimiser(AdamW)
                .inputs(layout)
                .save_format(&formats())
                .loss_fn(|output, target| output.sigmoid().squared_error(target))
                .build(|builder, stm, ntm| {
                    let l0f =
                        builder.new_weights("l0f", Shape::new(hidden, 768), InitSettings::Zeroed);
                    let mut l0 = builder.new_affine("l0", 768 * king_buckets, hidden);
                    l0.weights = l0.weights + l0f.repeat(king_buckets);
                    let l1 = builder.new_affine("l1", 2 * hidden, 1);
                    let us = l0.forward(stm).screlu();
                    let them = l0.forward(ntm).screlu();
                    l1.forward(us.concat(them))
                })
        ),
        _ => run_and_evaluate!(
            ValueTrainerBuilder::default()
                .dual_perspective()
                .optimiser(AdamW)
                .inputs(layout)
                .output_buckets(MaterialCount::<8>)
                .save_format(&formats())
                .loss_fn(|output, target| output.sigmoid().squared_error(target))
                .build(|builder, stm, ntm, output_buckets| {
                    let l0f =
                        builder.new_weights("l0f", Shape::new(hidden, 768), InitSettings::Zeroed);
                    let mut l0 = builder.new_affine("l0", 768 * king_buckets, hidden);
                    l0.weights = l0.weights + l0f.repeat(king_buckets);
                    let l1 = builder.new_affine("l1", 2 * hidden, 8);
                    let us = l0.forward(stm).screlu();
                    let them = l0.forward(ntm).screlu();
                    l1.forward(us.concat(them)).select(output_buckets)
                })
        ),
    };

    // The quantised net, in the engine's format.
    let ckpt = PathBuf::from(&out).join(format!("{net_id}-{superbatches}"));
    let mut net = std::fs::read(ckpt.join("quantised.bin"))
        .map_err(|e| format!("{}: {e}", ckpt.display()))?;
    if king_buckets > 1 {
        // Drop the factoriser listed after the engine's blocks (it is already
        // in every bucket's weights), and pad to 64 bytes as bullet does.
        let size = 2 * params(hidden, buckets, king_buckets);
        if net.len() < size {
            return Err(format!(
                "quantised.bin is {} bytes, short of the engine's {size}",
                net.len()
            ));
        }
        net.truncate(size);
        net.resize(size.next_multiple_of(64), 0);
    }
    std::fs::write(PathBuf::from(&out).join("net.nnue"), &net).map_err(|e| e.to_string())?;
    let checks: Vec<serde_json::Value> = fens
        .lines()
        .filter(|l| !l.is_empty())
        .zip(&trainer_cps)
        .map(|(fen, cp)| serde_json::json!({"fen": fen, "trainer_cp": cp}))
        .collect();
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
    if (final_net.hidden, final_net.buckets, final_net.king_buckets)
        != (hidden, buckets, king_buckets)
    {
        return Err(format!(
            "raw.bin read as {} hidden, {} output and {} king buckets, trained as {hidden}, {buckets} and {king_buckets}",
            final_net.hidden, final_net.buckets, final_net.king_buckets
        ));
    }
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

/// The (768 × k → h) × 2 → b SCReLU network in floats, as bullet trains it:
/// output in units of `eval_scale` centipawns. The width `h`, the output
/// buckets `b` and the king buckets `k` come from the file's size.
struct FloatNet {
    hidden: usize,
    buckets: usize,
    king_buckets: usize,
    /// Feature-major: feature `i`'s weights are `l0w[i * hidden..][..hidden]`,
    /// with any factoriser already added.
    l0w: Vec<f32>,
    l0b: Vec<f32>,
    /// Bucket-major, `[b][2h]`: each bucket's side-to-move half first.
    l1w: Vec<f32>,
    l1b: Vec<f32>,
    source: &'static str,
}

impl FloatNet {
    /// bullet's `raw.bin` (f32 weights, untransformed: a bucketed output
    /// layer's weights interleaved by bucket, and with king buckets the
    /// factoriser at the end) or a quantised net (`quantised.bin` without a
    /// factoriser, `net.nnue`: i16 in the engine's order, padded to 64 bytes),
    /// told apart by size, at any width and bucket counts the scorer handles.
    fn from_file(path: &Path) -> Result<FloatNet, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let shapes: Vec<(usize, usize, usize)> = (16..=MAX_HIDDEN)
            .step_by(16)
            .flat_map(|h| OUTPUT_BUCKETS.map(|b| (h, b)))
            .flat_map(|(h, b)| KING_BUCKETS.map(|k| (h, b, k)))
            .collect();
        let raw =
            |h: usize, b: usize, k: usize| 4 * (params(h, b, k) + if k > 1 { 768 * h } else { 0 });
        if let Some(&(h, b, k)) = shapes
            .iter()
            .find(|&&(h, b, k)| bytes.len() == raw(h, b, k))
        {
            let w: Vec<f32> = bytes
                .as_chunks::<4>()
                .0
                .iter()
                .map(|&c| f32::from_le_bytes(c))
                .collect();
            return Ok(FloatNet::from_params(&w, h, b, k, true, "float (raw.bin)"));
        }
        let quantised: Vec<(usize, usize, usize)> = shapes
            .iter()
            .copied()
            .filter(|&(h, b, k)| {
                bytes.len() >= 2 * params(h, b, k) && bytes.len() - 2 * params(h, b, k) < 64
            })
            .collect();
        match quantised[..] {
            [(h, b, k)] => {
                let n = params(h, b, k);
                let (qa, qb) = (f32::from(QA), f32::from(QB));
                let (l1, biases) = (768 * k * h + h, n - b);
                let w: Vec<f32> = bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .take(n)
                    .map(|&c| f32::from(i16::from_le_bytes(c)))
                    .enumerate()
                    .map(|(i, x)| match i {
                        _ if i < l1 => x / qa,
                        _ if i < biases => x / qb,
                        _ => x / (qa * qb),
                    })
                    .collect();
                Ok(FloatNet::from_params(
                    &w,
                    h,
                    b,
                    k,
                    false,
                    "quantised weights, dequantised to floats",
                ))
            }
            [] => Err(format!(
                "{}: {} bytes is not a float or quantised (768 × k → h) × 2 → b net for any h in 16..={MAX_HIDDEN}, b in {OUTPUT_BUCKETS:?} and k in {KING_BUCKETS:?} (h = 128, b = 1, k = 1: {} or {} bytes)",
                path.display(),
                bytes.len(),
                raw(128, 1, 1),
                2 * params(128, 1, 1)
            )),
            _ => Err(format!(
                "{}: {} bytes fits more than one quantised shape with padding: {quantised:?} (hidden, output buckets, king buckets)",
                path.display(),
                bytes.len()
            )),
        }
    }

    /// A net from its parameters in file order. `raw`: bullet's raw.bin,
    /// whose output weights are column-major `[2h][b]` rather than the
    /// engine's `[b][2h]`, and which, with king buckets, ends with the
    /// factoriser to add to every bucket's input weights.
    fn from_params(
        w: &[f32],
        hidden: usize,
        buckets: usize,
        king_buckets: usize,
        raw: bool,
        source: &'static str,
    ) -> FloatNet {
        let (l0w, rest) = w.split_at(768 * king_buckets * hidden);
        let (l0b, rest) = rest.split_at(hidden);
        let (l1w, rest) = rest.split_at(buckets * 2 * hidden);
        let (l1b, factoriser) = rest.split_at(buckets);
        let l1w = if raw {
            (0..buckets)
                .flat_map(|k| (0..2 * hidden).map(move |j| l1w[buckets * j + k]))
                .collect()
        } else {
            l1w.to_vec()
        };
        let mut l0w = l0w.to_vec();
        if raw && king_buckets > 1 {
            for (i, w) in l0w.iter_mut().enumerate() {
                *w += factoriser[i % factoriser.len()];
            }
        }
        FloatNet {
            hidden,
            buckets,
            king_buckets,
            l0w,
            l0b: l0b.to_vec(),
            l1w,
            l1b: l1b.to_vec(),
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
        // Each side's view from its own king, seen from its own side:
        // bulletformat stores the opponent's king square flipped.
        let (sb, sf) = king_view(self.king_buckets, usize::from(b.our_ksq()));
        let (nb, nf) = king_view(self.king_buckets, usize::from(b.opp_ksq()));
        let mut pieces = 0;
        for (piece, square) in *b {
            pieces += 1;
            let c = usize::from(piece & 8 > 0);
            let pc = 64 * usize::from(piece & 7);
            let sq = usize::from(square);
            let stm = sb + (([0, 384][c] + pc + sq) ^ sf);
            let ntm = nb + (([384, 0][c] + pc + (sq ^ 56)) ^ nf);
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
        let k = output_bucket(pieces, self.buckets);
        let w = &self.l1w[k * 2 * h..][..2 * h];
        let mut out = self.l1b[k];
        for i in 0..h {
            out += screlu(us[i]) * w[i] + screlu(them[i]) * w[h + i];
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
        let mut w = Vec::with_capacity(params(hidden, 1, 1));
        for f in 0..768 {
            for h in 0..hidden {
                w.push(l0(f, h));
            }
        }
        w.extend((0..hidden).map(|_| 0.0));
        w.extend((0..2 * hidden).map(l1));
        w.push(l1b);
        FloatNet::from_params(&w, hidden, 1, 1, false, "test")
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
            let n = params(hidden, 1, 1);
            let mut q = vec![0i16; n];
            q[0] = QA; // l0w → 1.0
            q[768 * hidden + hidden] = QB; // l1w[0] → 1.0
            q[n - 1] = QA * QB / 2; // l1b → 0.5
            let mut bytes: Vec<u8> = q.iter().flat_map(|w| w.to_le_bytes()).collect();
            bytes.resize(bytes.len().next_multiple_of(64), 0);
            let p = dir.join(format!("net{hidden}.nnue"));
            std::fs::write(&p, &bytes).unwrap();
            let net = FloatNet::from_file(&p).unwrap();
            assert_eq!((net.hidden, net.buckets), (hidden, 1));
            assert_eq!(net.l0w[0], 1.0);
            assert_eq!(net.l1w[0], 1.0);
            assert_eq!(net.l1b, [0.5]);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn output_buckets_are_bullets_material_count() {
        // bullet's MaterialCount<8>: (pieces - 2) / 4.
        for pieces in 2..=32 {
            assert_eq!(output_bucket(pieces, 8), (pieces - 2) / 4);
            assert_eq!(output_bucket(pieces, 1), 0);
        }
    }

    #[test]
    fn bucketed_files_read_in_both_orders() {
        // raw.bin interleaves the buckets (bullet's column-major [2h][8]);
        // the quantised file is the engine's [8][2h]. Both must give the same
        // bucket-major weights.
        let dir = std::env::temp_dir().join(format!("rc-trainer-buckets-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (h, b) = (128, 8);
        let n = params(h, b, 1);
        let ow = 768 * h + h;
        // Output weight j of bucket k is (k * 2h + j) / 4096 in floats.
        let want = |k: usize, j: usize| (k * 2 * h + j) as f32 / 4096.0;
        let mut raw = vec![0f32; n];
        for k in 0..b {
            for j in 0..2 * h {
                raw[ow + b * j + k] = want(k, j);
            }
            raw[n - b + k] = k as f32;
        }
        let p = dir.join("raw.bin");
        std::fs::write(
            &p,
            raw.iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<u8>>(),
        )
        .unwrap();
        let from_raw = FloatNet::from_file(&p).unwrap();
        assert_eq!((from_raw.hidden, from_raw.buckets), (h, b));
        let mut q = vec![0i16; n];
        for k in 0..b {
            for j in 0..2 * h {
                q[ow + k * 2 * h + j] = ((k * 2 * h + j) % 128) as i16;
            }
            q[n - b + k] = k as i16 * (QA * QB / 16);
        }
        let mut bytes: Vec<u8> = q.iter().flat_map(|w| w.to_le_bytes()).collect();
        bytes.resize(bytes.len().next_multiple_of(64), 0);
        let p = dir.join("net.nnue");
        std::fs::write(&p, &bytes).unwrap();
        let from_q = FloatNet::from_file(&p).unwrap();
        assert_eq!((from_q.hidden, from_q.buckets), (h, b));
        for k in 0..b {
            assert_eq!(from_raw.l1b[k], k as f32);
            assert_eq!(from_q.l1b[k], k as f32 / 16.0);
            for j in 0..2 * h {
                assert_eq!(
                    from_raw.l1w[k * 2 * h + j],
                    want(k, j),
                    "raw bucket {k} input {j}"
                );
                assert_eq!(
                    from_q.l1w[k * 2 * h + j],
                    ((k * 2 * h + j) % 128) as f32 / f32::from(QB),
                    "quantised bucket {k} input {j}"
                );
            }
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn forward_uses_the_bucket_of_the_piece_count() {
        // Zero hidden layer, so the output is the bucket's bias.
        let (h, b) = (128, 8);
        let mut w = vec![0f32; params(h, b, 1)];
        let n = w.len();
        for k in 0..b {
            w[n - b + k] = k as f32;
        }
        let net = FloatNet::from_params(&w, h, b, 1, false, "test");
        for (fen, pieces) in [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                32,
            ),
            ("8/5k2/3p4/1p1Pp2p/pP2Pp1P/P4P1K/8/8 b - - 0 1", 14),
            ("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1", 3),
        ] {
            let got = net.forward(&record(fen).unwrap());
            assert_eq!(got, output_bucket(pieces, b) as f32, "{fen}");
        }
    }

    #[test]
    fn an_ambiguous_quantised_size_is_refused() {
        // 880 hidden with 8 buckets and 896 with 1 are 18 bytes apart, inside
        // the padding: such a file can't be read without saying which it is.
        let dir = std::env::temp_dir().join(format!("rc-trainer-ambiguous-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("net.nnue");
        std::fs::write(&p, vec![0u8; 2 * params(896, 1, 1)]).unwrap();
        let err = FloatNet::from_file(&p).err().expect("refused");
        assert!(err.contains("more than one"), "{err}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_king_bucket_layout_is_the_engines() {
        let engine = include_str!("../../crates/search/src/nnue.rs");
        let start = engine
            .find("pub const KING_BUCKET_LAYOUT: [usize; 32] = [")
            .expect("the engine defines KING_BUCKET_LAYOUT");
        let body = &engine[start..];
        let body = &body[body.find('=').unwrap() + 1..body.find("];").unwrap()];
        let engine_layout: Vec<usize> = body
            .split(|c: char| !c.is_ascii_digit())
            .filter(|t| !t.is_empty())
            .map(|t| t.parse().unwrap())
            .collect();
        assert_eq!(engine_layout, KING_BUCKET_LAYOUT);
    }

    #[test]
    fn king_views_are_bullets_mirrored_buckets() {
        // bullet expands the 32-entry layout to 64 squares by mirroring files
        // e–h onto d–a, and flips with XOR 7 when the king is on files e–h.
        for ksq in 0..64 {
            let expanded = KING_BUCKET_LAYOUT[(ksq / 8) * 4 + [0, 1, 2, 3, 3, 2, 1, 0][ksq % 8]];
            let flip = if ksq % 8 > 3 { 7 } else { 0 };
            assert_eq!(king_view(4, ksq), (768 * expanded, flip), "square {ksq}");
            assert_eq!(king_view(1, ksq), (0, 0));
        }
    }

    #[test]
    fn raw_files_add_the_factoriser_to_every_king_bucket() {
        let (h, b, k) = (128, 8, 4);
        let n = params(h, b, k);
        let mut raw = vec![0f32; n + 768 * h];
        for (i, w) in raw.iter_mut().enumerate().take(768 * k * h) {
            *w = (i / (768 * h)) as f32; // bucket index
        }
        for (i, w) in raw[n..].iter_mut().enumerate() {
            *w = 0.25 + i as f32 / 1e6; // the factoriser
        }
        let dir =
            std::env::temp_dir().join(format!("rc-trainer-factoriser-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("raw.bin");
        std::fs::write(
            &p,
            raw.iter()
                .flat_map(|w| w.to_le_bytes())
                .collect::<Vec<u8>>(),
        )
        .unwrap();
        let net = FloatNet::from_file(&p).unwrap();
        assert_eq!((net.hidden, net.buckets, net.king_buckets), (h, b, k));
        for i in [0, 1, 768 * h - 1, 768 * h, 3 * 768 * h + 5] {
            let want = (i / (768 * h)) as f32 + 0.25 + (i % (768 * h)) as f32 / 1e6;
            assert_eq!(net.l0w[i], want, "weight {i}");
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_king_bucketed_net_is_mirror_symmetric_and_uses_the_kings_bucket() {
        // Random weights: a position and its left-right mirror give the same
        // output, and moving a king into another bucket changes it.
        let (h, b, k) = (64, 1, 4);
        let mut state = 0x9E37_79B9_7F4A_7C15u64;
        let mut rnd = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 2001) as f32 / 1000.0 - 1.0
        };
        let w: Vec<f32> = (0..params(h, b, k)).map(|_| rnd() * 0.3).collect();
        let net = FloatNet::from_params(&w, h, b, k, false, "test");
        let eval = |fen: &str| net.forward(&record(fen).unwrap());
        for (fen, mirrored) in [
            (
                "6k1/5ppp/8/8/8/8/PP6/1K6 w - - 0 1",
                "1k6/ppp5/8/8/8/8/6PP/6K1 w - - 0 1",
            ),
            (
                "8/2p5/3k4/8/8/4K3/5P2/8 b - - 0 1",
                "8/5p2/4k3/8/8/3K4/2P5/8 b - - 0 1",
            ),
        ] {
            assert!((eval(fen) - eval(mirrored)).abs() < 1e-5, "{fen}");
        }
        // White's king from g1 (bucket 0) to g3 (bucket 3).
        assert!(
            (eval("6k1/8/8/8/8/8/8/6K1 w - - 0 1") - eval("6k1/8/8/8/8/6K1/8/8 w - - 0 1")).abs()
                > 1e-4
        );
    }
}
