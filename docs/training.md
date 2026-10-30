# Training NNUE nets

The nets are trained with [`bullet`](https://github.com/jw1912/bullet) on the
self-play data in [datagen.md](datagen.md), which must have passed its audit
([datagen-runbook.md](datagen-runbook.md)) first. The trainer lives in
`trainer/`, a Cargo project of its own: bullet's GPU backends (CUDA, or Metal
on a Mac) never touch the engine's build. The network it trains is the one
[nnue.md](nnue.md) describes and the engine evaluates.

## The pipeline

```text
shards in S3 ──▶ prepare ──▶ train.data, val.data ──▶ train ──▶ net.nnue ──▶ check ──▶ data/nets/<net id>/
```

1. **`rusty-chess-trainer prepare --shards DIR --out DIR`**
   - **Split:** every *worker's* shards go wholly to validation or training,
     by `splitmix(fnv("run/task/worker") ^ splitmix(split_seed))`. A game
     never spans shards, so it never spans the split.
   - **Parse:** every record goes through bulletformat 1.8.0's text parser,
     the one bullet's `convert --from text` uses.
   - **Write:** bullet's 32-byte records, shuffled with a seeded generator
     (scattered to 256 bucket files, each Fisher-Yates shuffled, so memory
     stays small).
   - **Check:** the record count must equal the manifests' sum, or it fails.
   - **Outputs:** `split.json` (seeds, the validation workers, counts and the
     SHA-256 of both files) and `check.fens` (held-out positions for the
     check).
   - **Reproducible:** the same shards and seeds give byte-identical files.
     `bullet-utils shuffle` seeds itself from the clock, which is why the
     trainer shuffles itself.
2. **`rusty-chess-trainer train --data DIR --net-id ID --wdl λ --superbatches N [--checkpoint-every 4] [--val-positions 4000000]`**
   - **The network:** (768 → 128) × 2 → 1 with SCReLU and `Chess768` inputs,
     as in bullet's reference example.
   - **Loss:** `sigmoid(output)` against `λ · result + (1 − λ) · sigmoid(score / 400)`.
   - **Optimiser:** AdamW (bullet's defaults clip weights to ±1.98).
   - **Schedule:** 16,384 positions a batch and 6,104 batches a
     superbatch (100M positions). The learning rate starts at 0.001 and
     falls tenfold at 45% of the run.
   - **Checkpoints** every `--checkpoint-every` superbatches (and the last),
     each scored on held-out data after training (*Validation loss* below).
   - **Saves:** the quantized net as `net.nnue`, in the engine's format
     (`QA` 255, `QB` 64; bullet pads it to a multiple of 64 bytes, which the
     engine accepts).
   - **Check positions:** the trainer's own evaluation of `check.fens`, in
     `eval_check.json`.

   **The scale is fixed at 400**, the engine's `SCALE`. The engine multiplies
   the network's output by it, so training at another scale would rescale
   every evaluation, and with it every pruning margin tuned in centipawns.
   The trainer refuses any other value.
3. **Validation loss**, in `train` after training. bullet at the pinned
   commit prepares no validation pass (it prints *Validation data not
   currently implemented!*), so the trainer scores the checkpoints itself.
   - **What is scored:** each checkpoint's float weights (`raw.bin`, the
     network bullet trains, not the quantised one) on the first
     `--val-positions` records of `val.data` and on as many records of
     `train.data`. Both files are shuffled, so these are random samples, and
     the training sample makes the two numbers comparable. bullet's own loss
     is a running average while the weights move, so it is not.
   - **The loss** is bullet's: the mean over positions of
     `(sigmoid(output) − (λ · result + (1 − λ) · sigmoid(score / 400)))²`.
   - **Cost:** 4M + 4M positions take about 1 s per checkpoint on 4 threads
     of an M-series Mac. Allow about 5 s on a `g6.xlarge`'s 4 vCPUs, under
     a minute for the 10 checkpoints of a 40-superbatch run.
   - **The guard:** before scoring, the trainer's forward pass on the final
     weights must match bullet's own evaluation of the check positions
     within 0.5 cp (it matches to within 0.001), or `train` fails.
   - **Outputs:** `val_losses.txt` (`superbatch,train_loss_sample,val_loss`),
     a table in the log, and `validation.json` (the points, the sample
     sizes and the best superbatch). `training.json` includes it.
   - **Reading it:** the held-out loss above the training sample's and
     rising while the training loss falls is overfitting. Use fewer
     superbatches or more data. Comparing losses at different λ means
     nothing, since λ changes the target; strength decides λ.
   - **Noise:** 4M of generation 0's held-out positions put the loss within
     about 3·10⁻⁵ of the full 16.7M. Differences between checkpoints of one
     run are on the same positions, so they are finer than that.
4. **`nnue-eval --net net.nnue --check eval_check.json`** (in `crates/tools`)
   - **What it compares:** the engine's evaluation of the check positions,
     through its own loader and `Board::evaluate`, with the trainer's.
   - **It fails when:** any position is off by more than max(50 cp, 5%), or
     the mean by more than 20 cp.
   - **Why those bars:**
     - On the first pilot net, an independent implementation reproduced the
       trainer *exactly* from bullet's float weights (`raw.bin`), and the
       engine *exactly* (75 of 75) from the quantized net.
     - The whole gap between the two is quantization error: a mean of 9 cp
       and at most 28.
     - A layout, perspective or scale mismatch is hundreds. Swapping the
       output weights' halves, a perspective bug, gave a mean of 740.

## Running it

**On a GPU task:** the `Train` workflow runs `ci/train-task.sh` on
jwall-engineering-infra's `gpu` profile (one on-demand `g6.xlarge`, an NVIDIA
L4) from `main`:

```sh
gh workflow run Train --ref main -f net-id=net1 -f wdl=0.5 -f superbatches=40 -f generation=0
```

- `wdl` also takes a list (`0.25,0.5,0.75`): one net per λ, one at a time,
  since the account's G and VT quota fits one GPU instance.
- Each net's id is `<net-id>-l<λ×100>-r<run id>`, for example
  `net1-l050-r41234567890`. The GPU role can write `data/nets/` but not list
  it, so the run id is what keeps ids unique.
- The run summary lists each net with its records, losses and engine check.
- **Run `GPU smoke` first** after any change to the profile, the image or the
  trainer's build. It checks the GPU, CUDA, bullet's CUDA build of the
  trainer, and the dataset grants in both directions, in a few minutes.

1. It downloads the positions and manifests (not the game records).
2. It prepares, trains, and runs the check.
3. It writes `training.json`: the dataset, the split and its seeds, the
   schedule, the engine commit, the bullet commit and bulletformat version,
   the GPU, driver and CUDA version, the losses, and the check.
4. It uploads `net.nnue`, `training.json`, `eval_check.json`,
   `val_losses.txt`, `split.json`, `losses.txt` and `train.log` to
   `data/nets/<net id>/`.

A failed check uploads nothing. An existing net id is refused: nets are never
overwritten.

**On a Mac** (Apple silicon, Metal):

```sh
cd trainer && cargo build --release --features metal
target/release/rusty-chess-trainer prepare --shards /path/to/gen=0 --out /tmp/prepared
target/release/rusty-chess-trainer train --data /tmp/prepared --net-id net1-l050 --wdl 0.5 --superbatches 40 --out /tmp/net1
cargo run --release -p rusty-chess-tools --bin nnue-eval -- --net /tmp/net1/net.nnue --check /tmp/net1/eval_check.json
```

An M-series GPU trains this network at about 4.7 million positions a second,
so the 40-superbatch schedule (4 billion positions) takes about 15 minutes.
The whole pipeline also runs locally with `ci/train-task.sh --backend metal`.

## Scoring an existing net

`rusty-chess-trainer validate --net FILE --data DIR --wdl λ [--positions 4000000]`
scores any net on a prepared directory's `val.data`, and on as many records
of `train.data` if it is there. `FILE` is bullet's `raw.bin`, or a quantised
`net.nnue`, whose weights are dequantised to floats. To rebuild a net's exact
validation set, re-run `prepare` with its `split.json`'s seeds and
validation fraction. Only the validation workers' shards are needed: the
split is by worker.

Net 1 (`net1-l050-r41022503872`, 40 superbatches at λ 0.5), scored after the
fact on all 16,671,855 of its held-out positions: **validation loss
0.018925**, against 0.018898 on 1.5M of its training positions. A 0.14% gap,
so 40 passes over generation 0 did not overfit visibly.

## Then

1. **Load and test the net:** `setoption name EvalFile value net.nnue`, or
   `bench --eval-file`.
2. **SPRT it against PeSTO.** A first net typically gains 100 Elo or more.
3. **Run the λ experiment** ([datagen-runbook.md](datagen-runbook.md)) on the
   same prepared data, and keep the strongest net.
4. **Embed the winner** with `include_bytes!` and make it the default
   ([nnue.md](nnue.md)).
5. **Next generation:** generate generation 1 with it (`generation = 1` in
   `datagen.toml`).
