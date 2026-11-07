# NNUE nets

The ledger of every net trained, kept or not: what it was trained on and
how, what it measured, and how it played. It's the record for deciding what
to try next, so it keeps the failures too. The positions and labels each net
names are in [datasets.md](datasets.md); the architecture and file format in
[nnue.md](nnue.md); the training pipeline in [training.md](training.md).

Each net's own record (`training.json`, the losses, the validation curve, the
split, the log) is in `s3://jwall-engineering-rusty-chess-nnue/data/nets/<net id>/`.
Net ids are `<name>-l<λ×100>-r<Train run id>`.

The **Labels** column is the label set a net used for its generations. `O` means
every generation's original labels; a mix names each part, such as `N7` on
generations 0–2 and `O` on the rest. The keys, and a per-generation table for
every net, are in [datasets.md](datasets.md#which-labels-each-net-trained-on).

## Summary

| Net | Status | Data (generations, training positions) | Labels | Architecture | λ | Superbatches | Held-out loss | Engine check (mean / max cp) | Timed SPRT vs previous | Fixed-node SPRT vs previous | Ladder (Stockfish 19) |
|:--|:--|:--|:--|:--|--:|--:|--:|:--|:--|:--|:--|
| `net1-l050-r41022503872` | Default `v0.28.0`–`v0.29.0`; **plays generation 1** | 0 · 214.6M | `O` | (768 → 128) × 2 → 1 | 0.5 | 40 | 0.018925 (gen-0 held-out) | 8.72 / 29.82 | **+587.3 ± 82.7** vs PeSTO | +563.4 ± 93.8 | 2977 ± 32 (`v0.28.0`) |
| `net2-l050-r41110004652` | Default in `v0.30.0`; replaced by net2x80 (never played a generation) | 0 + 1 · 561.7M | `O` | (768 → 128) × 2 → 1 | 0.5 | 40 | 0.019273 (gen-0 + gen-1 held-out) | 5.52 / 20.32 | **+112.8 ± 21.1** vs net 1 | +134.4 ± 22.7 | 3044 ± 28 (`v0.30.0`) |
| `net2x80-l050-r41131366975` | Default `v0.31.0`–`v0.32.0`; **plays generation 2** | 0 + 1 · 561.7M | `O` | (768 → 128) × 2 → 1 | 0.5 | **80** | **0.019204** (same held-out set as net 2: 0.019273) | 5.67 / 22.38 | **+21.0 ± 8.9** vs net 2 | +23.5 ± 9.4 | 3025 ± 29 (`v0.31.0`), 3011 ± 32 (`v0.31.1`), 3034 ± 30 (`v0.32.0`) |
| `net2x80-l025-r41141703294` | Dropped (#137): λ sweep | 0 + 1 · 561.7M | `O` | (768 → 128) × 2 → 1 | **0.25** | 80 | not comparable (other target) | 4.68 / 20.10 | **−38.1 ± 12.8** vs net2x80 (H0) | −60.9 ± 16.7 (H0) | — |
| `net2x80-l075-r41145466004` | Dropped (#140): λ sweep | 0 + 1 · 561.7M | `O` | (768 → 128) × 2 → 1 | **0.75** | 80 | not comparable (other target) | 6.74 / 23.93 | **−3.2 ± 4.5** vs net2x80 (H0) | −4.1 ± 4.8 (H0) | — |
| `net3-l050-r41160495666` | Default `v0.33.0`–`v0.34.0`; replaced by net 4 (never played a generation) | 0 + 1 + 2 · 857.3M | `O` | (768 → 128) × 2 → 1 | 0.5 | 80 | 0.019862 (gen-0–2 held-out; cross-scored below) | 5.23 / 18.47 | **+21.8 ± 9.2** vs net2x80 | +32.3 ± 11.3 | 3051 ± 26 (`v0.33.0`), 3040 ± 32 (`v0.34.0`) |
| `net4-l050-r41165115607` | Default `v0.35.0`; replaced by net4w512 (never played a generation) | 0 + 1 + 2 · 857.3M | `O` | (768 → **256**) × 2 → 1 | 0.5 | 80 | **0.019443** (same held-out set as net 3: 0.019862) | 5.80 / 23.94 | **+74.2 ± 16.4** vs net 3 | +88.1 ± 16.4 | 3086 ± 33 (`v0.35.0`) |
| `net4w512-l050-r41170687764` | Default `v0.36.0`; **plays generation 3** | 0 + 1 + 2 · 857.3M | `O` | (768 → **512**) × 2 → 1 | 0.5 | 80 | **0.019146** (same held-out set: net 4 0.019443) | 11.72 / 34.10 | **+44.3 ± 13.1** vs net 4 | +73.6 ± 15.9 | 3076 ± 30 (`v0.36.0`) |
| `net5-l050-r41229287546` | Default `v0.37.0`–`v0.38.0`; **plays generation 4** | 0 + 1 + 2 + **3** · 1,465.5M | `O` | (768 → 512) × 2 → 1 | 0.5 | 80 | 0.019432 (gen-0–3 held-out; not comparable with net4w512's) | 7.77 / 26.67 | **+21.9 ± 9.2** vs net4w512 | +35.4 ± 11.7 | 3129 ± 32 (`v0.37.0`), 3114 ± 30 (`v0.37.1`), 3132 ± 29 (`v0.38.0`) |
| `net5x120-l050-r41237487627` | Dropped (#159): schedule experiment, inconclusive at the cap | 0 + 1 + 2 + 3 · 1,465.5M | `O` | (768 → 512) × 2 → 1 | 0.5 | **120** | **0.019399** (same held-out set as net 5: 0.019432) | 6.68 / 23.20 | +4.9 ± 3.9 vs net 5 (inconclusive) | −1.2 ± 4.2 (inconclusive) | — |
| `net5b8-l050-r41265636815` | Default `v0.39.0`–`v0.39.2`; replaced by net6 (never played a generation) | 0 + 1 + 2 + 3 · 1,465.5M | `O` | (768 → 512) × 2 → **8** (output buckets) | 0.5 | 80 | **0.019349** (same held-out set as net 5: 0.019432) | 6.14 / 21.71 | **+6.5 ± 4.4** vs net 5 | +12.4 ± 6.6 | 3127 ± 31 (`v0.39.0`), 3125 ± 29 (`v0.39.1`), 3120 ± 32 (`v0.39.2`) |
| `net6-l050-r41273387683` | Default `v0.40.0`–`v0.41.0`; replaced by net6k4 (never played a generation) | 0 + 1 + 2 + 3 + **4** · 1,932.3M | `O` | (768 → 512) × 2 → 8 (output buckets) | 0.5 | 80 | 0.019626 (gen-0–4 held-out; cross-scored against net5b8 below: −1.11% on gen 3, −1.63% on gen 4) | 7.58 / 26.35 | **+14.5 ± 7.1** vs net5b8 | +11.4 ± 6.3 | 3156 ± 30 (`v0.40.0`), 3119 ± 29 (`v0.41.0`) |
| `net6k4-l050-r41282883885` | Default `v0.42.0`–`v0.42.1`; **plays generation 5** | 0 + 1 + 2 + 3 + 4 · 1,932.3M | `O` | (768 × **4** → 512) × 2 → 8 (king and output buckets) | 0.5 | 80 | **0.019422** (same held-out set as net6: 0.019626) | 7.30 / 30.03 | **+21.7 ± 9.0** vs net6 | +40.0 ± 12.4 | 3166 ± 31 (`v0.42.0`), 3144 ± 29 (`v0.42.1`) |
| `net7-l050-r41363264869` | Default `v0.43.0`–`v0.44.3`; **will play generation 6** (not generated yet) | 0–5 · 2,238.2M | `O` | (768 × 4 → 512) × 2 → 8 (king and output buckets) | 0.5 | 80 | 0.019514 (gen-0–5 held-out; cross-scored against net6k4 below: −0.56% / −0.68% / −0.44% on gens 3 / 4 / 5) | 7.29 / 27.31 | **+6.8 ± 4.5** vs net6k4 | +5.1 ± 4.2 (inconclusive) | 3171 ± 27 (`v0.43.0`), 3146 ± 32 (`v0.44.0`), 3163 ± 33 (`v0.44.1`), 3166 ± 30 (`v0.44.2`) |
| `net7rl012-l050-r42054816508` | Default from `v0.45.0`; passed its SPRT (#202) | 0–5 · 2,238.2M | `N7` on 0–2, `O` on 3–5 | (768 × 4 → 512) × 2 → 8 (king and output buckets) | 0.5 | 80 | 0.019361 (its own labels; not comparable with net7's, cross-scored below) | 7.79 / 52.58 | **+16.3 ± 7.7** vs net7 | **+18.1 ± 8.1** | — |
| `net7rl012345-l050-r42398995769` | **Dropped:** failed its SPRT against net7rl012 (#210) | 0–5 · 2,238.2M | `N7` on 0–2, `N7r` on 3–5 | (768 × 4 → 512) × 2 → 8 (king and output buckets) | 0.5 | 80 | 0.019640 (its own labels; not comparable with net7rl012's, cross-scored below) | 8.22 / 35.56 | +1.9 ± 4.1 vs net7rl012 (inconclusive) | −5.2 ± 5.2 (H0) | — |
| `net7rl012w768-l050-r42411319602` | **Dropped:** failed its SPRT against net7rl012 at a clock (#214) | 0–5 · 2,238.2M | `N7` on 0–2, `O` on 3–5 | (768 × 4 → **768**) × 2 → 8 (king and output buckets) | 0.5 | 80 | **0.019168** (same held-out set as net7rl012: 0.019361) | 7.64 / 41.77 | −9.0 ± 6.4 vs net7rl012 (H0) | **+28.9 ± 10.2** (H1) | — |

**Comparing the columns:**
- **Held-out loss only compares on the same held-out data.** Net 2's
  includes generation 1, which has more draws and different labels, so its
  0.019273 is not worse than net 1's 0.018925. Score two nets on one set
  with `rusty-chess-trainer validate` to compare them.
- **The same positions aren't the same held-out data if the labels differ.**
  The `val.data` SHA-256 tells you: losses compare only when it matches
  ([datasets.md](datasets.md#prepared-training-sets)). The first 14 nets use
  `O`; net7rl012 is the first that doesn't.
- **Both SPRTs are required to merge, since #184.** The timed leg shows what
  the engine does at a clock, and the fixed-node leg isolates the net's
  knowledge from its speed. Through #180, every net above, only the timed leg
  gated. Only the owner can bypass the check, manually.
- **The SPRT is self-play against the previous net,** so it overstates gains
  against other engines. The ladder is the external scale.

## net1-l050-r41022503872

The first net, kept in #122 and released in `v0.28.0`.

| | |
|:--|:--|
| Data | Generation 0 (PeSTO): 214,568,489 training / 16,671,855 held-out records; whole workers split 297 / 23 at `val_fraction` 0.05, split seed 1, shuffle seed 1 |
| Network | (768 → 128) × 2 → 1, SCReLU, `Chess768`; QA 255, QB 64, SCALE 400 |
| Schedule | λ 0.5 (constant); 40 superbatches × 6,104 batches × 16,384 (4.0B positions, about 19 passes over the data); AdamW, LR 0.001, ×0.1 every 18 superbatches (0.0001 from 19, 0.00001 from 37) |
| Training | `Train` run [41022503872](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41022503872): NVIDIA L4, CUDA 13.2, bullet `da426a4`, engine `1505e24`. Prepare 217 s, training 194.5 s at 20.7M positions/s, 613 s to trained; about $0.20 |
| Loss | Training loss 0.0780 → 0.0186. Flat from about superbatch 13, with a step at the LR drop |
| Validation | Scored afterwards (#121; this bullet doesn't compute it). Held-out 0.018925 over all 16.7M records, against 0.018898 on 1.5M training records: a 0.14% gap, no visible overfitting |
| Engine check | 256 held-out positions: mean 8.72 cp, max 29.82, none beyond max(50 cp, 5%) |
| Bench | Signature 1201313 (PeSTO 1697503); EBF 1.90. 36% slower per node than PeSTO on the CI runner's baseline build; #125's vectorization won back +14.6% on `x86-64-v3` and about +30% on Apple silicon |
| SPRT vs PeSTO (#122) | Timed 5+0.05: **+587.3 ± 82.7**, H1 in 304 games (285 / 1 / 18). Fixed 200k nodes: +563.4 ± 93.8, H1 in 306 |
| Ladder | `v0.28.0`: 3000 ± 37 against Stockfish 17.1 at 2500–2950, then 2977 ± 32 against Stockfish 19 at 2800–3190 (#126). PeSTO's last release read 2726 ± 34 and 2602 ± 47 on the same two ladders, so the external gain is about +275 to +375 |

## net2-l050-r41110004652

The second net. **Only the data changed** from net 1: generation 1, played
by net 1, was added to generation 0.

| | |
|:--|:--|
| Data | Generations 0 + 1: 561,653,589 training / 37,941,221 held-out records; workers 537 / 39 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Generation 0's held-out workers are the same as net 1's |
| Network | Same as net 1 |
| Schedule | Same as net 1: λ 0.5, 40 superbatches, LR ×0.1 every 18 superbatches. 4.0B positions is about 7 passes over the data |
| Training | `Train` run [41110004652](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41110004652): NVIDIA L4, CUDA 13.2, bullet `da426a4`, engine `8a457b5`. Prepare 880 s, 2,247 s to trained; about $0.50 |
| Loss | Training loss 0.0681 → 0.0192 |
| Validation | Every 4 superbatches, on 4M held-out and 4M training records (table below). Best at the last checkpoint; training and held-out stay within 0.4% throughout, so no overfitting, and still improving at 40 |
| Engine check | 256 held-out positions: mean 5.52 cp, max 20.32 |
| Bench | Signature 1120617; fail-high-first 0.886, EBF 1.88 |
| SPRT vs net 1 (#129) | Timed 5+0.05: **+112.8 ± 21.1**, H1 in 542 games (240 / 70 / 232). Fixed 200k nodes: +134.4 ± 22.7, H1 in 502 |
| Ladder | `v0.30.0`: 3044 ± 28 |

| Superbatch | 4 | 8 | 12 | 16 | 20 | 24 | 28 | 32 | 36 | 40 |
|:--|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| Held-out loss | .019734 | .019602 | .019541 | .019509 | .019323 | .019310 | .019302 | .019297 | .019292 | .019273 |
| Training sample | .019697 | .019557 | .019486 | .019464 | .019266 | .019246 | .019244 | .019238 | .019229 | .019204 |

## net2x80-l050-r41131366975

Net 2's data and settings with **twice the schedule**: 80 superbatches, the
learning rate falling tenfold every 36 (0.0001 from superbatch 37, 0.00001
from 73). Net 2's held-out loss was still falling at its last checkpoint, so
this asks whether it was under-trained.

| | |
|:--|:--|
| Data | Same as net 2: generations 0 + 1, the same split and shuffle seeds, so the same 561,653,589 / 37,941,221 records and the **same held-out positions** |
| Schedule | λ 0.5; 80 superbatches (8.0B positions, about 14 passes over the data); LR 0.001, ×0.1 every 36 |
| Training | `Train` run [41131366975](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41131366975): NVIDIA L4, CUDA 13.2, bullet `da426a4`, engine `f29b399`. Prepare 868 s, 3,135 s to trained, about 23 s a superbatch at 4.3M positions/s (disk-bound: see [training.md](training.md#when-training-is-slow)); about $0.70 |
| Loss | Training loss 0.0681 → 0.0191 |
| Validation | Best at the last checkpoint, **0.019204**, against net 2's **0.019273** on the identical 4M held-out positions: 6.9·10⁻⁵ (0.36%) lower, more than net 2 gained over its whole second half (0.26%). Train and held-out stay within 0.4%: no overfitting at 14 passes |
| Engine check | 256 held-out positions: mean 5.67 cp, max 22.38 |
| Bench | Signature 1146589; fail-high-first 0.884, EBF 1.89 |
| SPRT vs net 2 (#134) | Timed 5+0.05: **+21.0 ± 8.9**, H1 in 2,356 games (633 / 491 / 1,232). Fixed 200k nodes: +23.5 ± 9.4, H1 in 2,062 |
| Ladder | `v0.31.0`: 3025 ± 29; `v0.31.1` (the same engine): 3011 ± 32. Net 2's `v0.30.0` read 3044 ± 28. The ladder can't see +21: two runs of one engine differ by 14, and each rating's interval is ±30 |

| Superbatch | 4 | 12 | 20 | 28 | 36 | 40 | 52 | 64 | 72 | 76 | 80 |
|:--|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| Held-out loss | .019729 | .019547 | .019502 | .019459 | .019446 | .019261 | .019237 | .019230 | .019223 | .019206 | .019204 |
| Training sample | .019690 | .019490 | .019453 | .019413 | .019406 | .019185 | .019167 | .019161 | .019159 | .019136 | .019132 |

**Reading the curve:**
- **Each LR drop gives the largest step.** The rate stayed at 0.001 to
  superbatch 36 and the loss was nearly flat by then; the drop at 37 is the
  big step, and the drop at 73 a small one.
- **Most of the gain is the longer high-LR phase.** At superbatch 40 this run
  (0.019261) is already level with net 2's final 0.019273, because it spent
  36 superbatches at 0.001 rather than 18.

## net7rl012w768-l050-r42411319602

net7rl012's recipe at 768 hidden neurons. **Dropped: much smarter per node, but
weaker at a clock.** Against net7rl012 (#214) it scored +28.9 ± 10.2 at fixed nodes
(H1) and −9.0 ± 6.4 timed (H0), so it fails the gate. The plan below was recorded
before any result existed, in #211, and is unchanged.

| | |
|:--|:--|
| Question | At 2.24B training positions, does a wider hidden layer, 768 neurons rather than 512, make a stronger net at a clock? |
| The one change from net7rl012 | `hidden` 768 instead of 512: 50% more first-layer weights (4,744,720 bytes against 3,163,200) and accumulator work per move |
| Held the same | net7rl012's data and labels (`g0-5·N7(0-2)`: `N7` on generations 0–2, `O` on 3–5), split, seeds, 8 output buckets, 4 king buckets, λ 0.5 and the 80-superbatch schedule. `trainer/`, `ci/train-task.sh` and the `Train` workflow have no change since `d25ce3f` except this PR's `hidden` description |
| Prepared set | `g0-5·N7(0-2)`: its `train.data` and `val.data` SHA-256s must equal net7rl012's (`9df1226c…07bd`, `ad1f1aa4…f8ef`) |
| Prerequisite | The engine reads 768-neuron nets (this PR), so `Train`'s engine check runs and the SPRT can load the net |
| Gate | H1 in both the timed and the fixed-node SPRT against net7rl012, the default. Anything else drops the net, unless the owner merges over the check with a stated reason |

```sh
gh workflow run Train --ref main -f net-id=net7rl012w768 -f generations=0,1,2,3,4,5 \
  -f labels=0=net7-l050-r41363264869-n5000,1=net7-l050-r41363264869-n5000,2=net7-l050-r41363264869-n5000 \
  -f hidden=768 -f output-buckets=8 -f king-buckets=4 -f wdl=0.5 -f superbatches=80
```

**Why 768, and the risk:**
- **The record so far:** going wider paid each time, and 512 beat 256 by
  +44.3 ± 13.1 timed (#152). But 512 was about 14% slower per node, and lost 30
  of its +74 fixed-node gain at a clock. 768 adds half as much work again, and its
  first layer, already past most L2 caches at 512, grows to 4.7 MB.
- **So the timed leg is the likely failure:** the fixed-node leg could pass on
  better judgement while the timed one doesn't. Both gate.

| | |
|:--|:--|
| Data | As net7rl012: generations 0–5, 2,238,186,465 training / 110,741,698 held-out records (94,149 shards); workers 1,336 / 72. Prepared set `g0-5·N7(0-2)`, `train.data` `9df1226c…07bd` and `val.data` `ad1f1aa4…f8ef`, both equal to net7rl012's |
| Network | (768 × 4 → 768) × 2 → 8; 4,744,768 bytes (4,744,720 plus 48 of padding); sha256 `430eb048…9ad0` (the file in S3 matches `training.json`) |
| Schedule | As net7rl012: 80 superbatches, LR 0.001, ×0.1 after 36 and 72 (the `schedule` objects are equal) |
| Training | `Train` run [42411319602](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42411319602), from `v0.46.0` (`1cd5486`): on-demand `g6.xlarge`, us-east-1a, NVIDIA L4, bullet `da426a4`. Prepare 1,121 s, 4,792 s to trained (net7rl012: 4,470). About $1.15, an estimate scaled from net7rl012's |
| Loss | Training loss 0.0723 → 0.0188 (net7rl012: 0.0723 → 0.0191) |
| Validation | **0.019168**, best at superbatch 80, against net7rl012's 0.019361 on the identical held-out set: **−1.00%**. Lower at every checkpoint from 60 to 80, and still falling at 80 |
| Engine check | 256 held-out positions: mean **7.64 cp**, max 41.77 (bar 20 on the mean; none beyond max(50 cp, 5%)). The first 768-neuron net the engine has read |

**Attempts:** one. The plan's command, from `1cd5486`, got an instance at the
first try, at 20:30:11 UTC on 2026-11-07.

**The plan's checks:**
- **Width:** `hidden` 768, with 8 output buckets and 4 king buckets.
- **Same data:** `labels`, `datasets`, `val_workers`, every record, worker and
  shard count, and the `train.data` and `val.data` SHA-256s equal net7rl012's.
- **Engine check:** `check_passed: true`.
- **Trainer:** `trainer/` and `ci/train-task.sh` have no change from `d25ce3f`
  to `1cd5486`. `train.yml` changed only in the `hidden` input's description
  (#211).

**Speed, a local measurement** (Apple M3 Max, `bench` at `1cd5486`, three runs
each, `--eval-file` for this net):

| Net | Bench signature | Nodes/s |
|:--|--:|:--|
| net7rl012 (default) | 976528 | 2.59M–2.88M |
| net7rl012w768 | 1013277 | 2.35M |

So it's about 12–18% slower per node here, and searches 3.8% more nodes on the
bench. That's about 23% longer to finish it. That's the timed-leg risk the plan
named. The SPRT PR's CI bench comment will be the record.

**Comparing it with net7rl012:** the held-out data is identical, so its own
validation loss is comparable with net7rl012's 0.019361 directly. A lower loss is
a reason to run the SPRT, not a strength result. **It's 1.00% lower,** between the
2.1% of 128 → 256 and the 1.5% of 256 → 512, on 2.6× the data of #152.

**Bench** (#214's CI comment, base `v0.46.0`): signature 1013277 (net7rl012:
976528), identical run to run and with `--features stats`.
- **Speed per node −13.9% ± 6.9%** and **time to depth +20.9% ± 9.4%**. The local
  measurement above (12–18% slower, about 23% longer) agrees.
- 4,750 instructions a node, against net7rl012345's 4,153 on the same bench
  positions (#210). net7rl012's own count wasn't in a comment.
- Fail-high-first 0.874 (net7rl012: 0.878). **EBF 2.010 (net7rl012: 1.84), over
  the bar of 2,** so **the performance gate failed** on `efficiency.max_ebf`. That
  bar is always enforced, so #214 couldn't merge without a declared exception.

**SPRT against net7rl012** ([run 42419939378](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42419939378),
#214, `3fb3cf2` against `main` at `1cd5486`; elo0 0, elo1 5, α = β = 0.05; on demand,
two `c8a.16xlarge` in us-east-1a; one attempt):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | −9.0 ± 6.4 | 0.3% | 4086 | 848 / 954 / 2284 | −2.95 | 70, 524, 952, 436, 61 | H0 |
| Fixed nodes, 200,000 per move | **+28.9 ± 10.2** | 100.0% | 1576 | 425 / 294 / 857 | 2.95 | 19, 122, 396, 211, 40 | H1 |

**The recorded gate isn't met:** the timed leg accepted H0, so the net is dropped,
as the plan said. The comment's reading: "Gate: fail: smarter per node, but the
extra cost per node eats the gain at a clock: do not merge; worth a `perf` pass to
make it cheaper and a re-test."

**Reading it:**
- **The plan named this risk, and it happened.** The fixed-node gain, +28.9, is
  the largest since king buckets (+40.0, net6k4). But about 14% fewer nodes a
  second and a larger tree cost more than that at 5+0.05. 512 kept +44 of its +74
  at a clock (#152); 768 kept none of its +29.
- **The knowledge is real; the cost is the problem.** A cheaper 768 net, through
  faster accumulator updates or a refresh cache, could be re-tested with this same
  file. Its bench signature would be unchanged by a pure speed change.
- **Not measured:** how much of the cost is the first layer leaving cache
  (4.7 MB against 3.2 MB) and how much is the extra arithmetic. Also not measured:
  whether a longer clock, where nodes matter less than judgement, would change the
  timed result.

## net7rl012345-l050-r42398995769

net7rl012's recipe with generations 3–5 on label set `N7r`. **Dropped: it failed
its SPRT against net7rl012 (#210), +1.9 ± 4.1 timed, inconclusive at 10,000 games,
and −5.2 ± 5.2 at fixed nodes, H0.** So relabelling generations 3–5 with
net7rl012 didn't make a stronger net. The plan below was recorded before any result
existed, in #206, and is unchanged.

| | |
|:--|:--|
| Question | net7rl012 gained from better scores on generations 0–2. Do better scores on generations 3–5 gain too? |
| The one change from net7rl012 | Generations 3–5 use label set `N7r`, net7rl012's scores at 5,000 nodes, in place of their originals from net4w512, net 5 and net6k4. That's 1,442.3M of 2,348.9M records (61.4%) |
| Held the same | Positions, results, split, seeds, generations 0–2's labels (`N7`), architecture, λ 0.5 and the schedule. The trainer, `ci/train-task.sh` and the `Train` workflow have no change since net7rl012's commit `d25ce3f` |
| Prepared set | `g0-5·N7(0-2)·N7r(3-5)` ([datasets.md](datasets.md#prepared-training-sets)). Its records, workers, shards and held-out workers must match `g0-5·N7(0-2)` |
| Prerequisite | `N7r` is complete for generations 3–5 and its audit is recorded (#205) |
| Gate | H1 in both the timed and the fixed-node SPRT against net7rl012, the default. Anything else drops the net, unless the owner merges over the check with a stated reason |
| Not tested | Two labellers stay in the data: net7 on generations 0–2 and net7rl012 on 3–5. Relabelling generations 0–2 with net7rl012 would be a separate experiment |

```sh
gh workflow run Train --ref main -f net-id=net7rl012345 -f generations=0,1,2,3,4,5 \
  -f labels=0=net7-l050-r41363264869-n5000,1=net7-l050-r41363264869-n5000,2=net7-l050-r41363264869-n5000,3=net7rl012-l050-r42054816508-n5000,4=net7rl012-l050-r42054816508-n5000,5=net7rl012-l050-r42054816508-n5000 \
  -f hidden=512 -f output-buckets=8 -f king-buckets=4 -f wdl=0.5 -f superbatches=80
```

| | |
|:--|:--|
| Data | Generations 0–5: 2,238,186,465 training / 110,741,698 held-out records (94,149 shards); workers 1,336 / 72 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Prepared set `g0-5·N7(0-2)·N7r(3-5)`: `train.data` `267501c5…182f`, `val.data` `556b16d9…a14a` |
| Network | (768 × 4 → 512) × 2 → 8, as net7rl012; 3,163,200 bytes; sha256 `c5b25752…5719` (the file in S3 matches `training.json`) |
| Schedule | As net7rl012: 80 superbatches, LR 0.001, ×0.1 after 36 and 72 |
| Training | `Train` run [42398995769](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42398995769), from `v0.45.1` (`d838b1c`): on-demand `g6.xlarge`, us-east-1a, NVIDIA L4, bullet `da426a4`. Prepare 1,151 s, 4,365 s to trained (net7rl012: 4,470). About $1.05, an estimate scaled from net7rl012's |
| Loss | Training loss 0.0756 → 0.0195 |
| Validation | 0.019640 on its own held-out set, best at superbatch 80. Not comparable with net7rl012's 0.019361: the labels differ |
| Engine check | 256 held-out positions: mean **8.22 cp**, max 35.56 (bar 20 on the mean; none beyond max(50 cp, 5%)) |

**Attempts:** one. The plan's command, from `d838b1c`, got an instance at the
first try, at 17:59:56 UTC on 2026-11-07.

**The plan's checks:**
- **Labels:** `training.json`'s `labels` and `datasets` name `N7` for generations
  0–2 and `N7r` for generations 3–5, both under `selfplay/relabel/`.
- **Same positions:** `val_workers` and all record, worker and shard counts equal
  net7rl012's, and the `val.data` SHA-256 differs from net7rl012's (`ad1f1aa4…f8ef`).
- **Trainer:** `trainer/`, `ci/train-task.sh` and the `Train` workflow have no
  change from `d25ce3f` to `d838b1c`.

**Comparing it with net7rl012:** as before, each net's own held-out loss isn't
comparable, since the labels differ. Score both quantised nets with
`rusty-chess-trainer validate` on identical held-out positions:

| Held-out set, target | net7rl012 | net7rl012345 | Change |
|:--|--:|--:|--:|
| Generations 3–5, results only (`--wdl 1.0`), the one target the label sets share | 0.064857 | **0.064441** | −0.64% |
| Generations 0–2, results only (`--wdl 1.0`) | **0.067404** | 0.067454 | +0.07% |
| Generations 3–5 under `O`, λ 0.5 | **0.019208** | 0.019234 | +0.14% |
| Generations 3–5 under `N7r`, λ 0.5 | 0.019732 | **0.019618** | −0.57% |
| Generations 0–2 under `N7`, λ 0.5, which both trained on | **0.019619** | 0.019725 | +0.54% |

Expect each net to do better on its own labels for generations 3–5. The
results-only rows and generations 0–2 are the fairer tests, and the SPRT decides.

**What it showed:** much smaller differences than net7rl012 against net7, and
mixed:
- **Generations 3–5:** net7rl012345 fits the game results better, by 0.64%. It
  does better on its own labels and about the same on the originals (+0.14%,
  against net7rl012's +7.05% on generations 0–2's originals). `N7r` sits close to
  the originals ([datasets.md](datasets.md#net7rl012-l050-r42054816508-n5000)), so
  little changed there.
- **Generations 0–2:** net7rl012345 is a little worse on the `N7` labels both nets
  trained on (+0.54%), and level on results.

So it isn't a clear improvement anywhere but generations 3–5's results. That
doesn't drop the net, since the SPRT decides.

**How it was measured:** a local measurement on an Apple M3 Max with the trainer
built from `d25ce3f` (`--features metal`; `trainer/` is unchanged at `d838b1c`),
scoring every held-out record. Three sets are net7rl012's, reused unchanged. The
new one, generations 3–5 under `N7r`, was prepared with seed 1 from the 26
held-out workers' shards in the `N7r` audit's download:

| Set | Records | Workers | Shards | `val.data` |
|:--|--:|--:|--:|:--|
| Generations 3–5 under `N7r` | 61,372,315 | 26 | 2,458 | `91a532ef…5a4c` |

Its records, workers and shards equal generations 3–5 under `O`. Compared record
by record, all 61,372,315 records are identical apart from the score, and the
scores differ in 60,108,686 (97.9%). net7rl012's four scores on the reused sets
reproduce [its comparison](#net7rl012-l050-r42054816508) exactly.

**Not measured:**
- **Noise:** these differences have no error bars, and this is one training run
  of each net. They're smaller than net7rl012's against net7.
- **Bench and speed:** not run on this net yet; that comes with the SPRT PR.
- **The `Train` artifact's copy of the net:** only the S3 copy was hashed.

**Bench** (#210's CI comment, base `v0.45.1`): signature 886973 (net7rl012:
976528), identical run to run and with `--features stats`; **time to depth −9.8% ±
2.7%**, speed per node +0.7% ± 3.1%; fail-high-first 0.881 (net7rl012: 0.878); EBF
1.89 (net7rl012: 1.84, worse, inside the bar of 2); 4,153 instructions a node. The
performance gate passed.

**SPRT against net7rl012** ([run 42406536564](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42406536564),
#210, `8eefd3f` against `main` at `ad0a9fb`; elo0 0, elo1 5, α = β = 0.05; on demand,
two `c8a.16xlarge` in us-east-1a; one attempt, no interruptions):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | +1.9 ± 4.1 | 81.6% | 10000 | 2211 / 2157 / 5632 | 0.26 | 153, 1155, 2333, 1203, 156 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | −5.2 ± 5.2 | 2.3% | 6414 | 1417 / 1514 / 3483 | −2.96 | 138, 721, 1552, 692, 104 | H0 |

**The recorded gate isn't met:** neither leg accepted H1, so the net is dropped, as
the plan said. The comment's reading: "Gate: fail: no gain at a clock: do not
merge."

**Reading it:**
- **Per node it played worse:** the fixed-node interval, −10.4 to −0.1 (−5.25 ± 5.18 in the log), sits
  below zero. The timed leg's +1.9 ± 4.1 includes zero and −2. The two legs'
  intervals overlap, and the net is no slower per node, so they don't show a real
  gain at a clock. No experiment here tests a gain smaller than about 5.
- **The held-out comparison had warned:** small, mixed differences, and slightly
  worse on generations 0–2's `N7` labels (+0.54%).
- **Not measured:** why. `N7r` sits much closer to the originals than `N7` did
  ([datasets.md](datasets.md#net7rl012-l050-r42054816508-n5000)), and it mixes two
  labellers with `N7`. This is one training run.

## net7rl012-l050-r42054816508

net7's recipe with generations 0–2 on label set `N7`. **It beat net7 in both SPRT
legs (+16.3 ± 7.7 timed, +18.1 ± 8.1 at fixed nodes; #202), so better labels on the
same positions made a stronger net.** The plan below was recorded before any result
existed, in #198, and is unchanged.

| | |
|:--|:--|
| Question | With the same positions and results, do better scores make a stronger net? |
| The one change | Generations 0–2 use label set `N7`, net7's scores at 5,000 nodes, in place of their originals from PeSTO, net 1 and net2x80. That's 906.6M of 2,348.9M records (38.6%) |
| Held the same | Positions, results, split, seeds, generations 3–5's labels, architecture, λ 0.5 and the schedule |
| Prepared set | `g0-5·N7(0-2)` ([datasets.md](datasets.md#prepared-training-sets)). Its records, workers, shards and held-out workers must match net7's `g0-5·O` |
| Prerequisite | `N7` is complete for generations 0–2 and its audit is recorded |
| Gate | H1 in both the timed and the fixed-node SPRT against net7. Anything else drops the net, unless the owner merges over the check with a stated reason |

```sh
gh workflow run Train --ref main -f net-id=net7rl012 -f generations=0,1,2,3,4,5 \
  -f labels=0=net7-l050-r41363264869-n5000,1=net7-l050-r41363264869-n5000,2=net7-l050-r41363264869-n5000 \
  -f hidden=512 -f output-buckets=8 -f king-buckets=4 -f wdl=0.5 -f superbatches=80
```

| | |
|:--|:--|
| Data | Generations 0–5: 2,238,186,465 training / 110,741,698 held-out records (94,149 shards); workers 1,336 / 72 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Prepared set `g0-5·N7(0-2)`: `train.data` `9df1226c…07bd`, `val.data` `ad1f1aa4…f8ef` |
| Network | (768 × 4 → 512) × 2 → 8, as net7; 3,163,200 bytes; sha256 `74af5b46…70ce` (the file in S3 matches `training.json`) |
| Schedule | As net7: 80 superbatches, LR 0.001, ×0.1 after 36 and 72 |
| Training | `Train` run [42054816508](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42054816508), from `v0.44.4` (`d25ce3f`): on-demand `g6.xlarge`, us-east-1a, NVIDIA L4, bullet `da426a4`. Prepare 1,124 s, 4,470 s to trained (net7: 4,027). About $1.05, an estimate scaled from net7's cost |
| Loss | Training loss 0.0723 → 0.0191 |
| Validation | 0.019361 on its own held-out set, best at superbatch 80. Not comparable with net7's 0.019514: the labels differ. Scorer against bullet: 0.0024 cp |
| Engine check | 256 held-out positions: mean **7.79 cp**, max 52.58 (bar 20 on the mean; none beyond max(50 cp, 5%)) |

**Attempts:** the first real run of `labels` (#190). All used the plan's command,
from `d25ce3f`. The three refused attempts started no instance, so they cost
nothing to run. `Train` tries each zone once and then fails.

| Run | Started (UTC) | Outcome |
|:--|:--|:--|
| [42041209288](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42041209288) | 2026-11-05 14:02 | No on-demand `g6.xlarge` in us-east-1a, b, c, d or f |
| [42047772456](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42047772456) | 14:43 | The same |
| [42049345865](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42049345865) | 14:53 | The same |
| [42054816508](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42054816508) | 15:25 | Launched in us-east-1a; succeeded |

**The plan's checks:**
- **Labels:** `training.json`'s `labels` and `datasets` name `N7` for generations
  0–2 and `selfplay/v1/` for generations 3–5.
- **Same positions:** `val_workers` and all record, worker and shard counts equal
  net7's, and the `val.data` SHA-256 differs from net7's (`4dc5921e…b99e`).
- **Trainer:** `trainer/` has no change from `b2497b7` to `d25ce3f`. In
  `ci/train-task.sh` only the label-set handling changed.

**Comparing it with net7:** their held-out positions match but their held-out
labels don't, so each net's own held-out loss **isn't comparable**. Score both
quantised nets with `rusty-chess-trainer validate` on each of these, on identical
positions:

| Held-out set, target | net7 | net7rl012 | Change |
|:--|--:|--:|--:|
| Generations 0–2, results only (`--wdl 1.0`), the one target the label sets share | 0.067977 | **0.067404** | −0.84% |
| Generations 3–5, results only (`--wdl 1.0`) | 0.065801 | **0.064857** | −1.43% |
| Generations 0–2 under `O`, λ 0.5 | **0.019634** | 0.021017 | +7.05% |
| Generations 0–2 under `N7`, λ 0.5 | 0.019982 | **0.019619** | −1.82% |
| Generations 3–5 under `O`, λ 0.5 | 0.019470 | **0.019208** | −1.35% |

Expect each net to do better on its own labels for generations 0–2, which says
little on its own. The results-only rows and generations 3–5 are the fairer
tests, and the SPRT decides.

**What it showed:** each net did better on its own labels, as expected. net7rl012
also did better on all three fairer rows, including generations 3–5 under the
labels both nets trained on. That's a reason to run the SPRT, not a strength
result.

**How it was measured:** a local measurement on an Apple arm64 machine, scoring
every held-out record with the trainer from `d25ce3f` (`--features metal`; the
scorer is the CPU float network). Each set was prepared with seed 1 from only its
held-out workers' shards, so it isn't a slice of either net's `val.data`:

| Set | Records | Workers | Shards | `val.data` |
|:--|--:|--:|--:|:--|
| Generations 0–2 under `O` | 49,369,383 | 46 | 1,980 | `4b4a0b73…7137` |
| Generations 0–2 under `N7` | 49,369,383 | 46 | 1,980 | `4f3e14a6…6122` |
| Generations 3–5 under `O` | 61,372,315 | 26 | 2,458 | `d6453319…ae48` |

The record and worker totals match the 110,741,698 and 72 in `training.json`. The
two generation 0–2 sets were compared record by record: all 49,369,383 are
identical outside the score, and the scores differ in 48,736,581 (98.7%), so the
comparison wasn't of one file against itself.

**Not measured:**
- **Noise:** these differences have no error bars, and this is one training run
  of each net.
- **Why results-only loss fell:** `N7`'s scores are larger than the originals
  ([datasets.md](datasets.md#net7-l050-r41363264869-n5000)), and a net's
  confidence affects this loss, so it may not all be better judgement.
- **The `Train` artifact's copy of the net:** only the S3 copy was hashed.

**Bench** (#202's CI comment, base `v0.44.4`): signature 976528 (net7: 1031896),
identical run to run and with `--features stats`; **time to depth −7.0% ± 2.3%**,
speed per node +1.8% ± 2.5%; fail-high-first 0.878 (net7: 0.875); EBF 1.841 (net7:
1.805, worse, inside the bar of 2). The performance gate passed.

**SPRT against net7** ([run 42076828557](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42076828557),
#202, `900d227` against `main` at `d25ce3f`; elo0 0, elo1 5, α = β = 0.05; on demand;
one attempt):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+16.3 ± 7.7** | 100.0% | 3058 | 747 / 604 / 1707 | 2.96 | 58, 284, 713, 405, 69 | H1 |
| Fixed nodes, 200,000 per move | **+18.1 ± 8.1** | 100.0% | 2654 | 651 / 513 / 1490 | 2.96 | 34, 271, 603, 361, 58 | H1 |

**The recorded gate is met:** H1 in both legs. Self-play against the previous net
overstates gains, and this is one training run of the net. Whether the owner merges
#202 and what the ladder reads are still open.

## net7-l050-r41363264869

net6k4's recipe (512 hidden, 8 output buckets, 4 king buckets, λ 0.5, 80
superbatches) with **generation 5 added**: the only change is the data, and
the first net trained on data a king-bucketed net played.

| | |
|:--|:--|
| Data | Generations 0–5: 2,238,186,465 training / 110,741,698 held-out records (94,149 shards); workers 1,336 / 72 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Generations 0–4's held-out workers are net6k4's; generation 5 adds 13,062,136 held-out records. 16% more training data than net6k4. Prepared data: `train.data` `f551e37e…d5f2`, `val.data` `4dc5921e…b99e` |
| Network | (768 × 4 → 512) × 2 → 8, as net6k4; 3,163,200 bytes; sha256 `0866bde1…1b6f` (the `Train` artifact and the embedded file are byte-identical) |
| Schedule | As net6k4: 80 superbatches (8.0B positions, about 3.6 passes), LR 0.001, ×0.1 after 36 and 72 |
| Training | `Train` run [41363264869](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41363264869), from `v0.42.1` (`b2497b7`): NVIDIA L4, CUDA 13.2, bullet `da426a4`. Prepare 1,129 s, 4,027 s to trained (net6k4: 3,891), about $0.95 |
| Loss | Training loss 0.0659 → 0.0192 |
| Validation | 0.019514 on generations 0–5's held-out set, 1.58% above the training sample (net6k4: 1.67%). Not comparable with net6k4's 0.019422: the set adds generation 5. Scorer against bullet: 0.0027 cp. Best at superbatch 80 |
| Engine check | 256 held-out positions: mean **7.29 cp**, max 27.31 |
| Bench | Signature 1031896 (net6k4: 1046838), identical run to run; fail-high-first 0.875 (net6k4: 0.879), **EBF 1.805** (net6k4: 1.855). Paired locally against net6k4 over 6 runs: speed per node +1.3%, **time to depth −2.6% ± 2.1%** |
| SPRT vs net6k4 (#180) | Run [41369992341](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41369992341). Timed 5+0.05: **+6.8 ± 4.5**, LOS 99.8%, H1 in 8,088 games (1,789 / 1,631 / 4,668; LLR 2.95). Fixed 200k nodes: **+5.1 ± 4.2**, LOS 99.2%, inconclusive at the 10,000-game cap (2,271 / 2,123 / 5,606; LLR 2.42). Runner: +0.1% ± 2.3% speed per node, −1.5% ± 2.3% time to depth; EBF 1.86 → 1.81, fail-high-first 87.9% → 87.5%; 4,106 instructions a node (net6k4: 4,143) |
| Ladder | `v0.43.0`: **3171 ± 27** ([41374337957](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41374337957)), `v0.44.0`: 3146 ± 32 ([41384790229](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41384790229)), `v0.44.1`: 3163 ± 33 ([41542113707](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41542113707)), `v0.44.2`: 3166 ± 30 ([41780137077](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41780137077)), the same net and bench signature. net6k4 read 3166 ± 31 and 3144 ± 29: the ladder can't separate them |

| Superbatch | 4 | 20 | 36 | 40 | 60 | 72 | 80 |
|:--|--:|--:|--:|--:|--:|--:|--:|
| **net7 held-out (generations 0–5)** | .020502 | .020182 | .020149 | .019728 | .019594 | .019564 | .019514 |
| net7 training sample | .020353 | .020004 | .019949 | .019485 | .019319 | .019261 | .019210 |
| net6k4 held-out (generations 0–4), for shape only | .020390 | .020087 | .020026 | .019630 | .019505 | .019469 | .019422 |

**Against net6k4 on identical held-out positions** (each generation prepared
alone with split seed 1, which rebuilds the held-out workers both nets
excluded; both quantised nets scored on the first 8M with `rusty-chess-trainer
validate`):

| Held-out set | net6k4 | net7 | Change |
|:--|--:|--:|--:|
| Generation 3 | 0.018981 | **0.018875** | **−0.56%** |
| Generation 4 | 0.019770 | **0.019636** | **−0.68%** |
| Generation 5 (net6k4 never saw it) | 0.020131 | **0.020042** | **−0.44%** |

**Reading it:**
- **Better everywhere, by about half of net6's data gain** (−1.11% / −1.63%
  over net5b8). Generation 5 added 16% more data where generation 4 added 32%,
  so a smaller step is what more of the same data predicts.
- **No slower:** the search needs fewer nodes (EBF 1.855 → 1.805) at the
  same speed per node.
- **The timed SPRT kept it: a real gain, but the smallest of any net so far.**
  +6.8 from 16% more data, against +14.5 from 32% (net6) and +21.9 from 71%
  (net 5).

**How the SPRT ran:**
1. **Spot reclaimed the fixed-node leg** of the first attempt at 01:26 UTC on
   2026-11-02, at +5.3 ± 4.8, LLR 1.92 (recorded in #184). The timed leg had
   finished with H1 by 01:37.
2. **#180 merged at 01:42 on the timed H1,** the gate at the time, before any
   fixed-node result existed.
3. **The fixed-node leg was re-run after the merge** (01:47–02:23, attempt 2 of
   the same run) and reached the 10,000-game cap, inconclusive. The sticky
   comment on #180 holds the result; the PR description's SPRT section still
   says "pending", so this entry is the record.
4. **#184 then moved both legs to on demand** and made the check require H1 in
   both. net7 was not re-judged. Under that check its inconclusive fixed-node
   leg would have needed the owner to merge over it.

**Reading the pair:** the comment's automatic reading is "no smarter per node,
but faster", which the numbers don't support on either side. The fixed-node
leg is inconclusive, not level: +5.1 with LOS 99.2% and most of the way to H1.
The runner's speed and time to depth are both within noise. Both estimates are
consistent with about +5 to +7, close to the test's `elo1` of 5, which is
why both legs needed 8,000 games or more.

## net6k4-l050-r41282883885

net6's recipe with **4 king buckets**: each side's inputs are indexed by its
own king's region, mirrored with the king on files e–h, with a factoriser
([king-buckets.md](king-buckets.md), #172, #173). The only change: same data,
width, output buckets, λ, schedule and seeds, and byte-identical prepared
data (net6's `train.data` and `val.data` SHA-256s), so the **same held-out
positions** as net6.

| | |
|:--|:--|
| Data | As net6: generations 0–4, 1,932,314,071 training / 97,679,562 held-out records |
| Network | (768 × 4 → 512) × 2 → 8, SCReLU, `Chess768` with `KING_BUCKET_LAYOUT`, mirrored; factoriser merged; QA 255, QB 64, SCALE 400; 3,163,200 bytes; sha256 `82a7f986…df33` |
| Schedule | As net6: λ 0.5, 80 superbatches, LR 0.001, ×0.1 after 36 and 72; `l0w` and `l0f` clipped to ±0.99 |
| Training | `Train` run [41282883885](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41282883885), from `v0.41.0`: prepare 1,006 s, **3,891 s to trained (net6: 3,768, +3%)**: the 4× input weights cost little on the L4. The GPU instance ran 66 minutes; GitHub marked the job done an hour after the task finished, a runner delay with no instance running. About $0.90 |
| Validation | **0.019422** against net6's 0.019626 on identical held-out positions: **−1.04%**. Scorer against bullet: 0.0012 cp. Held-out sits **1.67%** above the training sample (net6: 0.89%): the 4× input weights fit the training data more closely, but held-out still improved by the most yet. Best at superbatch 80, the lead still widening |
| Engine check | 256 held-out positions: mean **7.30 cp**, max 30.03, none beyond max(50 cp, 5%). The first full-size king-bucketed net through the engine |
| Bench | Signature 1046838 (net6: 1099777), identical run to run; **fail-high-first 0.879** (net6: 0.873), **EBF 1.855** (net6: 1.937). Paired locally against net6 over 8 runs: **11% slower per node, 7.4% ± 3.7% longer to depth**: rebuilds on king moves and a 4× weight table. 1.88M nodes/s locally (local bar 2M, informational; CI's bar is 1M) |
| SPRT vs net6 (#175) | Timed 5+0.05: **+21.7 ± 9.0**, LOS 100%, H1 in 2,210 games (584 / 446 / 1,180). Fixed 200k nodes: **+40.0 ± 12.4**, H1 in 1,220 (368 / 228 / 624). Runner: **−10.8% ± 2.3% speed per node, +6.8% ± 2.9% time to depth**; EBF 1.94 → 1.86, fail-high-first 87.3% → 87.9%; 4,143 instructions a node (net6: 3,799, +9%) |
| Ladder | `v0.42.0`: **3166 ± 31**, the highest yet; `v0.42.1` (the same net): 3144 ± 29 (net6: 3156 and 3119) |

| Superbatch | 4 | 20 | 36 | 40 | 60 | 72 | 80 |
|:--|--:|--:|--:|--:|--:|--:|--:|
| net6 held-out | .020451 | .020118 | .020035 | .019728 | .019678 | .019661 | .019626 |
| **net6k4 held-out** | .020390 | .020087 | .020026 | .019630 | .019505 | .019469 | .019422 |
| Change | −0.30% | −0.15% | **−0.04%** | −0.50% | −0.88% | −0.98% | **−1.04%** |

**Reading it:**
- **The buckets pay only once the learning rate drops.** At LR 0.001 the lead
  shrank to almost nothing (−0.04% at superbatch 36); after the first drop it
  grew at every checkpoint, to −1.04%. The per-region weights need the finer
  steps to specialise, and the lead was still widening at 80, so king-bucketed
  nets may want a longer low-LR phase (unlike net5x120's single-view net).
- **The largest same-data gain measured:** −1.04%, against net5b8's −0.43%
  from output buckets and net6's −1.11% / −1.63% from a new generation.
- **Paid for in speed, and worth it:** 6.8% longer to depth on the runner,
  yet **+40.0 at fixed nodes became +21.7 at a clock**: about half the
  knowledge survives the cost. A refresh cache (a `perf` change, signature
  unchanged) would win back some of the 10.8% per node.

## net6-l050-r41273387683

net5b8's recipe (512 hidden, 8 output buckets, λ 0.5, 80 superbatches) with
**generation 4 added**: the only change is the data.

| | |
|:--|:--|
| Data | Generations 0–4: 1,932,314,071 training / 97,679,562 held-out records (81,394 shards); workers 1,152 / 64 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Generations 0–3's held-out workers are net5b8's; generation 4 adds 12 held-out workers, 28,343,627 records. 32% more training data than net5b8 |
| Network | (768 → 512) × 2 → 8, as net5b8; 803,904 bytes; sha256 `282c21ba…2b1f` |
| Schedule | As net5b8: 80 superbatches (8.0B positions, about 4.1 passes, against 5.5); LR 0.001, ×0.1 after 36 and 72 |
| Training | `Train` run [41273387683](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41273387683), from `v0.39.1`: prepare 988 s, 3,768 s to trained (net5b8: 3,240; the 62 GB training file is about four times the instance's memory), 65 minutes for the job, about $0.90 |
| Validation | 0.019626 on generations 0–4's held-out set, 0.89% above the training sample (net5b8: 0.54%). Not comparable with net5b8's 0.019349: the set adds generation 4, the most drawn and hardest to predict, and generation 4 is a larger share of the held-out records (29%) than of the training records (24%), which inflates the gap. Best at superbatch 80 |
| Engine check | 256 held-out positions: mean **7.58 cp**, max 26.35, none beyond max(50 cp, 5%) |
| Bench | Signature 1099777 (net5b8: 1048958), identical run to run; **fail-high-first 0.873** (bar 0.87; net5b8 0.8702), EBF 1.937 (net5b8: 1.887); 2.10M nodes/s locally on an idle machine |
| SPRT vs net5b8 (#170) | Timed 5+0.05: **+14.5 ± 7.1**, LOS 100%, H1 in 3,256 games (768 / 632 / 1,856). Fixed 200k nodes: **+11.4 ± 6.3**, H1 in 4,604 (1,101 / 950 / 2,553). Runner: +1.9% ± 1.3% speed per node, +2.9% ± 1.3% time to depth; EBF 1.89 → 1.94; 3,799 instructions a node |
| Ladder | `v0.40.0`: 3156 ± 30; `v0.41.0` (the same net): 3119 ± 29. Two runs of one net 37 apart: the ladder's ±40 |

**Against net5b8 on identical held-out positions.** `prepare` puts a worker
in the held-out set by a hash of its key and the split seed alone, so
preparing one generation by itself with seed 1 rebuilds exactly the held-out
workers both nets excluded (19,966,552 records for generation 3, 28,343,627
for generation 4, matching their splits). Both quantised nets were scored on
the first 8M of each with `rusty-chess-trainer validate`:

| Held-out set | net5b8 | net6 | Change |
|:--|--:|--:|--:|
| Generation 3 (data both nets trained on) | 0.019447 | **0.019230** | **−1.11%** |
| Generation 4 (net5b8 never saw generation 4) | 0.020378 | **0.020046** | **−1.63%** |

**Reading it:**
- **Better on old data too:** −1.11% on generation 3, which both nets trained
  on, so net6 learned more in general, not just generation 4's style of play.
- **The largest gain measured on identical positions at 512** so far: 2.5–4×
  net5b8's −0.43% from buckets. Data stays the main lever.
- **The SPRT decides.**

## net5b8-l050-r41265636815

Net 5 with **8 output buckets**: the output layer has 8 sets of weights, one
chosen per position by its piece count, `(pieces − 2) / 4` (#162, #163). The
only change: same data, width, λ, schedule and seeds, and byte-identical
prepared data (net 5's `train.data` and `val.data` SHA-256s), so the **same
held-out positions** as net 5.

| | |
|:--|:--|
| Data | As net 5: generations 0–3, 1,465,509,939 training / 69,335,935 held-out records |
| Network | (768 → 512) × 2 → 8, SCReLU, `Chess768`, output bucket `(pieces − 2) / 4`; QA 255, QB 64, SCALE 400; 803,904 bytes; sha256 `1317c14a…3d74` |
| Schedule | As net 5: λ 0.5, 80 superbatches, LR 0.001, ×0.1 after 36 and 72 |
| Training | `Train` run [41265636815](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41265636815), from `v0.38.0`, alongside generation 4's datagen (the GPU quota is separate): 3,240 s to trained (net 5: 2,937, +10%), 56 minutes for the job, about $0.75 |
| Validation | **0.019349** against net 5's 0.019432 on identical held-out positions: **−0.43%**, ahead at every checkpoint (−0.37% to −0.50%). Held-out sits 0.54% above the training sample (net 5: 0.56%): no extra overfitting from the 7 more output layers |
| Engine check | 256 held-out positions: mean **6.14 cp**, max 21.71 (net 5: 7.77 / 26.67). The first full-size bucketed net through the engine |
| Bench | Signature 1048958 (net 5: 1101129); **fail-high-first 0.8702, at the 0.87 bar** (net 5: 0.875); EBF 1.887 (net 5: 1.818). About 2.09M nodes/s locally; #162 measured the bucket choice at +0.6% instructions a node |
| SPRT vs net 5 (#164) | Timed 5+0.05: **+6.5 ± 4.4**, LOS 99.8%, H1 in 8,324 games (1,859 / 1,702 / 4,763; LLR 2.96). Fixed 200k nodes: **+12.4 ± 6.6**, H1 in 4,184 (963 / 814 / 2,407). Runner: −0.8% ± 1.0% speed per node, **−3.9% ± 0.9% time to depth**; EBF 1.82 → 1.89, fail-high-first 87.5% → 87.0%; 3,779 instructions a node (net 5: 3,725) |
| Ladder | `v0.39.0`: 3127 ± 31, `v0.39.1`: 3125 ± 29, `v0.39.2`: 3120 ± 32 (the same net), against net 5's 3132 ± 29 (`v0.38.0`) |

| Superbatch | 4 | 20 | 36 | 40 | 72 | 80 |
|:--|--:|--:|--:|--:|--:|--:|
| net 5 held-out | .020165 | .019840 | .019801 | .019525 | .019469 | .019432 |
| **net5b8 held-out** | .020065 | .019753 | .019708 | .019440 | .019384 | .019349 |
| Change | −0.50% | −0.44% | −0.47% | −0.43% | −0.43% | −0.43% |

**Reading it:**
- **A constant lead from the first checkpoint,** not a late one: the buckets
  let the net fit every phase of the schedule better, rather than training
  longer.
- **The largest same-data gain so far** at 512: net5x120's longer schedule
  gave −0.17% and nothing at fixed nodes; net2x80's gave −0.36% and +21.0.
- **The SPRT decided: kept, but smaller than the loss suggested.** −0.43%
  held-out became +6.5 timed and +12.4 at fixed nodes, against net2x80's
  +21.0 from −0.36%. Equal held-out gains are not equal Elo: the buckets
  mostly sharpened phases the search already handled, and the timed leg
  needed 8,324 games to decide.

## net5x120-l050-r41237487627

Net 5 trained for **120 superbatches** instead of 80: the only change is the
schedule. **Dropped (#159):** inconclusive on both SPRT legs at the cap. Byte-identical prepared data (the same `train.data` and `val.data`
SHA-256s), so the **same held-out positions** as net 5.

| | |
|:--|:--|
| Data | As net 5: generations 0–3, 1,465,509,939 training / 69,335,935 held-out records |
| Network | (768 → 512) × 2 → 1, as net 5; sha256 `c24e670f…6a95` |
| Schedule | λ 0.5; **120 superbatches** (12.0B positions, about 8.2 passes); LR 0.001 for 1–54, 0.0001 for 55–108, 0.00001 for 109–120 (step 54, the default 45%) |
| Training | `Train` run [41237487627](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41237487627), from `v0.37.0`: 3,599 s to trained (net 5: 2,937), 62 minutes for the job, about $0.85. A first dispatch named `net6` (run 41236717509) was cancelled before training and renamed to follow the `net2x80` convention |
| Validation | **0.019399** against net 5's 0.019432 on identical held-out positions: **−0.17%**. Best at 120. Held-out sits 0.60% above the training sample (net 5: 0.56%) |
| Engine check | 256 held-out positions: mean **6.68 cp**, max 23.20 (net 5: 7.77 / 26.67) |
| Bench | Signature 1130959 (net 5: 1101129); fail-high-first 0.875; **EBF 2.049** (net 5: 1.818), past the 2.0 bar. The PR declared a 2.1 exception to be kept only on an SPRT pass; with the PR closed, the bar stays 2.0 |
| SPRT vs net 5 (#159) | Timed 5+0.05: **+4.9 ± 3.9**, LOS 99.2%, inconclusive at the 10,000-game cap (2,148 / 2,008 / 5,844; LLR 2.45). Fixed 200k nodes: **−1.2 ± 4.2**, LOS 28.0%, inconclusive at the cap (LLR −1.88). Runner: −7.5% ± 9.7% speed per node, +11.7% ± 12.7% time to depth. The owner's rule, set before the run: both legs must pass, or the PR closes |

| Superbatch | 36 | 52 | 72 | 80 | 108 | 120 |
|:--|--:|--:|--:|--:|--:|--:|
| net 5 held-out (LR drops after 36 and 72) | .019801 | — | .019469 | .019432 | — | — |
| **net5x120 held-out** (LR drops after 54 and 108) | .019798 | .019755 | .019463 | .019452 | .019436 | .019399 |

**Reading it:**
- **Where the gain comes from:** the longer first phase ended 0.23% lower
  than net 5's (.019755 at 52 against .019801 at 36), and the lead held to the
  end at −0.17%.
- **The middle phase flattened** from about superbatch 88 (.019444 → .019436
  at 108), so stretching it bought little; the last drop gave the same 0.19%
  as net 5's.
- **Smaller than the last schedule gain:** net2x80's −0.36% held-out was
  worth +21.0 against net 2. Here −0.17% was worth +4.9 at a clock and
  nothing at fixed nodes: the net isn't measurably smarter per node.
- **Why it was dropped:** the fixed-node leg, which isolates the net's
  knowledge, is level (−1.2 ± 4.2). The small timed edge, if real, is
  inside what a different tree shape can give at a clock, and it came with
  EBF 2.05.

## net5-l050-r41229287546

net4w512's recipe (512 hidden, λ 0.5, 80 superbatches) with **generation 3
added**: the only change is the data. It's the first net trained on data a
512-neuron net played.

| | |
|:--|:--|
| Data | Generations 0–3: 1,465,509,939 training / 69,335,935 held-out records; workers 908 / 52 at `val_fraction` 0.05, split seed 1, shuffle seed 1. Generations 0–2's held-out workers are the same as nets 3, 4 and net4w512's; generation 3 adds about 20M held-out records. 71% more training data than net4w512 |
| Network | (768 → 512) × 2 → 1, SCReLU, `Chess768`; QA 255, QB 64, SCALE 400; 789,568 bytes; sha256 `acd8e049…906b` |
| Schedule | As net4w512: 80 superbatches × 6,104 × 16,384 (8.0B positions, about 5.5 passes over the data against net4w512's 9.3); LR 0.001, ×0.1 every 36 |
| Training | `Train` run [41229287546](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41229287546), from `v0.36.0` (`f7605e5`): prepare 739 s, 2,937 s to trained (net4w512: 2,300), 51 minutes for the whole job. The 47 GB training file is three times the instance's memory. About $0.70 |
| Loss | Training loss 0.0684 → 0.0193 |
| Validation | **0.019432** on generations 0–3's held-out set, against 0.019325 on the training sample: +0.55%, the same gap as net4w512 (+0.53%). Best at superbatch 80, still falling. Not comparable with net4w512's 0.019146: this held-out set includes generation 3, which is more drawn |
| Engine check | 256 held-out positions: mean **7.77 cp**, max 26.67, none beyond max(50 cp, 5%). Run in training, the first time for a 512-neuron net |
| Bench | Signature 1101129 (net4w512: 1203006), identical run to run; fail-high-first 0.875 (bar 0.87), **EBF 1.818** (net4w512: 1.906); about 2.13–2.20M nodes/s locally, the same width and speed as net4w512 |
| SPRT vs net4w512 (#155) | Timed 5+0.05: **+21.9 ± 9.2**, H1 in 2,288 games (603 / 459 / 1,226). Fixed 200k nodes: +35.4 ± 11.7, H1 in 1,370. Runner: +1.4% ± 1.9% speed per node (unchanged), **−9.7% ± 1.7% time to depth** |
| Ladder | `v0.37.0`: **3129 ± 32**, against net4w512's 3076 ± 30 (`v0.36.0`): the highest yet. Rungs: 2800 86.0%, 2950 64.0%, 3100 57.5%, 3190 45.0% ([Rating run 41236545440](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41236545440)). Within about 60 of the ladder's top rung |

| Superbatch | 4 | 20 | 40 | 60 | 80 |
|:--|--:|--:|--:|--:|--:|
| **net 5 held-out (generations 0–3)** | .020165 | .019840 | .019525 | .019475 | .019432 |
| net4w512 held-out (generations 0–2), for shape only | .019761 | .019499 | .019225 | .019181 | .019146 |

**Not measured yet:** the two nets on identical held-out positions
(`rusty-chess-trainer validate` on each held-out set, as for net4w512 against
net 4). The timed SPRT is the verdict either way.

## net4w512-l050-r41170687764

Net 4's recipe and data at **512 hidden neurons**. Same held-out positions
as nets 3 and 4.

| | |
|:--|:--|
| Network | (768 → 512) × 2 → 1, SCReLU, `Chess768`; QA 255, QB 64, SCALE 400; 789,568 bytes |
| Training | `Train` run [41170687764](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41170687764), from `v0.35.0` (`f8495b0`): 2,300 s to trained, against net 4's 1,726. **The GPU is now the limit:** 6.8M positions/s, 14.8 s a superbatch, against 11.7M at 128 and 256, where loading was the limit. About $0.55. A first dispatch (run 41170474213, from `ddc636c`, the commit before the `v0.35.0` release) was cancelled 25 seconds in, before anything trained; the reason wasn't recorded |
| Validation | **0.019146**: −1.5% against net 4, −3.6% against net 3, on the identical held-out positions. Best at superbatch 76, level at 80 |
| Engine check | Skipped in training (the engine read up to 256), then run once it read 512: mean **11.72 cp**, max 34.10, inside the bars. Twice the 128/256 nets' error: twice the rounded quantized terms in each output sum |
| Bench | Signature 1203006; fail-high-first 0.880, EBF 1.906 (net 4: 1.86); about 2.10M nodes/s locally, against net 4's about 2.49M (about 14% slower per node on an M-series core), close to the 2M local speed bar |
| SPRT vs net 4 (#152) | Timed 5+0.05: **+44.3 ± 13.1**, H1 in 1,056 games (300 / 166 / 590). Fixed 200k nodes: +73.6 ± 15.9, H1 in 666. Runner: −11.4% speed per node, +14.1% time to depth |
| Ladder | `v0.36.0`: 3076 ± 30, against net 4's 3086 ± 33 (`v0.35.0`): level within the ladder's ±40, despite +44 in the SPRT |

| Superbatch | 4 | 20 | 40 | 60 | 80 |
|:--|--:|--:|--:|--:|--:|
| net 3 (128) held-out | .020502 | .020159 | .019925 | .019892 | .019862 |
| net 4 (256) held-out | .020058 | .019756 | .019515 | .019476 | .019443 |
| **net4w512 (512) held-out** | .019761 | .019499 | .019225 | .019181 | .019146 |

**Against net 4 on identical held-out positions** (quantized nets):

| Held-out set | Loss against | net 4 (256) | net4w512 (512) | Change |
|:--|:--|--:|--:|--:|
| Generations 0 + 1 (37,941,221) | the training target (λ 0.5) | 0.019082 | **0.018867** | −1.1% |
| | game results only | 0.069456 | **0.069029** | −0.6% |
| | search scores only | 0.008275 | **0.008271** | −0.05% |
| Generation 2 (11,428,162) | the training target (λ 0.5) | 0.020849 | **0.020465** | −1.8% |
| | game results only | 0.073069 | **0.072365** | −1.0% |
| | search scores only | 0.003310 | **0.003247** | −1.9% |

**Reading it:**
- **Better on all six again, but by less.** 128 → 256 gained 2.1% held-out,
  and 256 → 512 gains 1.5%.
- **The first sign the data limits the width:** held-out now sits about 0.5%
  *above* the training sample (0.019146 against 0.019045). That gap was
  −0.5% for net 3 and level for net 4.
- **It's still improving, not overfitting.** More data would help the wide
  nets most.

## net4-l050-r41165115607

Net 3's recipe and data at **256 hidden neurons** instead of 128: the first
change of architecture. Same generations, seeds and split, so the **same
held-out positions** as net 3.

| | |
|:--|:--|
| Data | As net 3: generations 0–2, 857,263,237 training / 49,369,383 held-out records |
| Network | (768 → 256) × 2 → 1, SCReLU, `Chess768`; QA 255, QB 64, SCALE 400; 394,816 bytes |
| Schedule | As net 3: λ 0.5, 80 superbatches, LR ×0.1 every 36 |
| Training | `Train` run [41165115607](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41165115607) (`--hidden 256`, #146): 1,726 s to trained, against net 3's 1,692. Training is bound by data loading, not the GPU, so twice the hidden layer cost about nothing |
| Validation | **0.019443**, against net 3's 0.019862 on the identical held-out positions: **−2.1%**. Ahead from the first checkpoint (superbatch 4, 0.020058, about level with net 3's 40). Best at 80, held-out equal to the training sample (0.019443 against 0.019459): no overfitting |
| Engine check | Skipped in training (the engine read 128 only), then run once the engine read 256 (#149): 256 held-out positions, mean 5.80 cp, max 23.94 |
| Bench | Signature 1190429; EBF 1.86 (net 3: 1.88); **fail-high-first 0.879, under the 0.88 bar** (net 3: 0.886); about 2.49M nodes/s locally, against net 3's about 2.75M (about 7% slower per node on an M-series core) |
| SPRT vs net 3 (#150) | Timed 5+0.05: **+74.2 ± 16.4**, H1 in 708 games (247 / 98 / 363), on the PR's final commit (with the fail-high-first exception). The first run, before the exception commit: +67.3 ± 16.4 in 774. Fixed 200k nodes: +88.1 ± 16.4. Runner: −7.5% speed per node, +14.1% time to depth |
| Ladder | `v0.35.0`: 3086 ± 33, against net 3's 3040 ± 32 (`v0.34.0`) and 3051 ± 26 (`v0.33.0`) |

| Superbatch | 4 | 20 | 40 | 60 | 80 |
|:--|--:|--:|--:|--:|--:|
| net 3 (128) held-out | .020502 | .020159 | .019925 | .019892 | .019862 |
| **net 4 (256) held-out** | .020058 | .019756 | .019515 | .019476 | .019443 |

**Against net 3 on identical held-out positions** (quantized nets):

| Held-out set | Loss against | net 3 (128) | net 4 (256) | Change |
|:--|:--|--:|--:|--:|
| Generations 0 + 1 (37,941,221) | the training target (λ 0.5) | 0.019447 | **0.019082** | **−1.9%** |
| | game results only | 0.070158 | **0.069456** | −1.0% |
| | search scores only | 0.008301 | **0.008275** | −0.3% |
| Generation 2 (11,428,162) | the training target (λ 0.5) | 0.021443 | **0.020849** | **−2.8%** |
| | game results only | 0.074124 | **0.073069** | −1.4% |
| | search scores only | 0.003443 | **0.003310** | −3.9% |

**Better on all six, on identical data,** so it's the extra capacity. Unlike
net 3 against net2x80, nothing is mixed.

## net3-l050-r41160495666

net2x80's recipe on **generations 0 + 1 + 2**, adding generation 2 (307M
positions played by net2x80). Only the data changed.

| | |
|:--|:--|
| Data | Generations 0–2: 857,263,237 training / 49,369,383 held-out records; workers 722 / 46 at `val_fraction` 0.05, split seed 1, shuffle seed 1. The 39 held-out workers of generations 0 + 1 are the same as net 2's and net2x80's, plus 7 from generation 2 |
| Schedule | λ 0.5; 80 superbatches (8.0B positions, about 9 passes over the data); LR 0.001, ×0.1 every 36 |
| Training | `Train` run [41160495666](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41160495666): NVIDIA L4, CUDA 13.2, bullet `da426a4`, engine `66818d8`; 1,692 s to trained, about 31 minutes in all, about $0.45. Downloads 8:20 (the three generations one after another), prepare 7:38 (parallel, #138: 2.07M records/s against 1.02M before), training 11.7M positions/s on local NVMe (8.5 s a superbatch) |
| Validation | Best at the last checkpoint, **0.019862**, flat over the last 4 superbatches: 80 is about right for this much data. Held-out about 0.5% *below* the training sample throughout (an easier held-out mix, not memorization) |
| Engine check | 256 held-out positions: mean 5.23 cp, max 18.47 |
| Bench | Signature 1128154; fail-high-first 0.886, EBF 1.88 |
| SPRT vs net2x80 (#145) | Timed 5+0.05: **+21.8 ± 9.2**, H1 in 2,280 games (604 / 461 / 1,215). Fixed 200k nodes: +32.3 ± 11.3, H1 in 1,522 |
| Ladder | `v0.33.0`: 3051 ± 26 and `v0.34.0`: 3040 ± 32, against net2x80's 3034 ± 30 (`v0.32.0`) |

**Against net2x80 on identical held-out positions** (neither net trained on
them; quantized nets, `rusty-chess-trainer validate`):

| Held-out set | Loss against | net2x80 | net 3 | Change |
|:--|:--|--:|--:|--:|
| Generations 0 + 1 (37,941,221) | the training target (λ 0.5) | **0.019255** | 0.019447 | +1.0% |
| | game results only | 0.071103 | **0.070158** | −1.3% |
| | search scores only | **0.006973** | 0.008301 | +19% |
| Generation 2 (11,428,162) | the training target (λ 0.5) | 0.022368 | **0.021443** | **−4.1%** |
| | game results only | 0.076266 | **0.074124** | −2.8% |
| | search scores only | **0.003151** | 0.003443 | +9% |

**Reading it:**
- **Net 3 predicts game results better on both sets.**
- **On the newest, strongest data it's clearly better** on the training target.
- **It agrees less with the older generations' search scores.** Those were
  labelled by PeSTO's and net 1's searches.
- **net2x80's low score loss on generation 2 is partly circular:** its own
  search produced those scores.
- **The λ sweep showed losses don't decide strength.** These point the right
  way, and the SPRT decides.

## The λ sweep: net2x80 at λ 0.25 and 0.75

**The question:** should the training target lean more on the game result or
on the 5,000-node search score? The target is
`λ × result + (1 − λ) × sigmoid(score / 400)`.

**Setup:** each candidate is net2x80 with only λ changed:
- generations 0 + 1, the same split and shuffle seeds, so the same
  561,653,589 / 37,941,221 records;
- 80 superbatches, LR ×0.1 every 36, bullet `da426a4`.

The protocol was decided before the results: each candidate must win a timed
SPRT against net2x80, and if both did, they'd play each other.

**Training:** the first runs with scratch on local NVMe. About 12M
positions/s, 27 minutes a net, against net2x80's 52.

| | `net2x80-l025-r41141703294` | net2x80 (λ 0.5) | `net2x80-l075-r41145466004` |
|:--|--:|--:|--:|
| `Train` run | [41141703294](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41141703294) | [41131366975](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41131366975) | [41145466004](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41145466004) |
| Engine check (mean / max cp) | 4.68 / 20.10 | 5.67 / 22.38 | 6.74 / 23.93 |
| Validation curve | best at 80, no overfitting | best at 80 | best at 80, train and held-out within 0.6% |
| **Held-out loss against game results only** | 0.075665 | 0.071103 | **0.068570** |
| **Held-out loss against search scores only** | **0.003933** | 0.006973 | 0.011693 |
| Bench signature | 1130254 | 1146589 | 975671 |
| **Timed SPRT 5+0.05 against net2x80** | **−38.1 ± 12.8**, H0 in 1,198 games (233 / 364 / 601) | — | **−3.2 ± 4.5**, H0 in 9,526 games (2,248 / 2,336 / 4,942), LOS 8.2% |
| Fixed 200k nodes against net2x80 | −60.9 ± 16.7, H0 in 830 games | — | −4.1 ± 4.8, H0 in 7,772 games |
| PR | #137, closed | #134, merged | #140, closed |

**How the cross-losses were measured:** on all 37.9M held-out positions,
re-prepared locally from the 39 held-out workers with the same seeds (the
record count matches exactly), scoring the quantized nets with
`rusty-chess-trainer validate --wdl 1.0` (results) and `--wdl 0.0`
(scores). Each λ's own training loss isn't comparable, because the targets
differ.

**What it showed:**
- **λ 0.5 is at or near the best for our data.** Strength falls off sharply
  toward the search score (λ 0.25: about −40 timed, −60 at fixed nodes), and
  is flat to slightly worse toward the result (λ 0.75: −3, LOS 8%).
- **Matching the search's scores isn't the same as playing well.** λ 0.25
  matched the 5,000-node scores 44% better than net2x80 and played clearly
  worse. At about depth 7 those scores have blind spots the game result
  doesn't, and a net that imitates them inherits the blind spots.
- **Predicting results better isn't enough either.** λ 0.75 predicted game
  results 3.6% better than net2x80 and didn't play better. The result labels
  each of a game's ~95 positions with one outcome, often decided much later,
  and about 43% of games are draws, so they're noisy.
- **Neither held-out loss predicts strength.** The SPRT decides, and we keep
  using it as the gate.

## Before net 1

- **Pilot nets** were trained locally on Apple Metal while building the
  trainer (#115).
- **They proved the pipeline, not strength.** An independent implementation
  reproduced the trainer's float evaluation and the engine's quantized one
  exactly, and the gap between them was quantization alone (mean 9 cp,
  max 28).
- **They weren't kept,** and none was tested in games.

## What we've learned

| What | Evidence | What it suggests |
|:--|:--|:--|
| **Data is the lever so far** | Net 2 differs from net 1 only in its data (net 1's games added, 2.6× the positions) and gained +113 Elo. Net 1 had gained +587 over PeSTO | Keep cycling generations. The next one is generation 2, played by net 2 |
| **Training is cheap; data is the cost** | A 128-neuron net trains in 3–4 minutes, and preparing data takes longer than training. A generation costs about $15–20 on demand (less on spot) | Run training experiments freely. Make data deliberately and audit it before training |
| **Net 2 was still improving at superbatch 40** | Held-out loss fell at every checkpoint, best at the last, with no train/held-out gap | Try a longer schedule (80 superbatches, LR drop at 36) on the same data |
| **Self-play SPRT overstates external Elo** | Net 1: +587 in the SPRT against PeSTO, about +275 to +375 on the Stockfish ladders | Judge nets by the timed SPRT, and track absolute strength on the ladder |
| **The ladder needs moving as nets improve** | `v0.28.0` scored 89% on `UCI_Elo` 2650; the rungs moved to 2800–3190 (#126). 3190 is Stockfish's limit | After about 3300, anchor to full Stockfish at a fixed node budget |
| **Training slows about 5× once the data outgrows the GPU instance's memory** | Net 1's 6.9 GB training file trained at 20.7M positions/s. Net 2's 18 GB file trained at 4.3M, the EBS root volume pinned at 131 MB/s: both the gp3 volume and the `g6.xlarge`'s EBS bandwidth are 125 MB/s | Put scratch on the instance's local NVMe (an infra change, proposed). Until then, budget about 23 s a superbatch on generations 0 + 1; diagnosing it is in [training.md](training.md#when-training-is-slow) |
| **λ 0.5 is at or near the best** | The λ sweep: 0.25 lost −38.1 ± 12.8, 0.75 lost −3.2 ± 4.5, both against net2x80 | Keep λ 0.5. Revisit only if the labels change a lot (much deeper search, or far more decisive games). A λ *schedule* (lower early, higher late) is the one variant worth a later try |
| **Matching a loss isn't playing well** | λ 0.25 matched search scores 44% better and played worse; λ 0.75 predicted results 3.6% better and didn't play better | Use held-out losses as health checks and on identical data only; let the SPRT decide |
| **Training length was worth +21** | net2x80 (80 superbatches) beat net 2 (40) by +21.0 ± 8.9 timed, from a 0.36% lower held-out loss on the same positions | 80 superbatches is the baseline. With more data, check the curve to decide if more is needed |
| **The ladder can't see gains of about +20** | net2x80, +21 against net 2 in the SPRT, read 3025 and 3011 on two ladder runs, against net 2's 3044 (each ±30) | Treat a single ladder run as ±40. A dedicated-instance ladder with about 400 games a rung would read ±19 |
| **Held-out loss doesn't compare across datasets** | Net 2's 0.019273 against net 1's 0.018925, on different held-out data, while net 2 is +113 stronger | Compare nets on one held-out set |
| **Width was the biggest step since net 2** | On the same data: 128 → 256 gained **+74.2 ± 16.4** timed (#150), 256 → 512 **+44.3 ± 13.1** (#152). Held-out loss fell 2.1%, then 1.5% | Width pays, with diminishing returns. 1024 would be about 2× slower per node again; try it only with much more data |
| **Wider nets cost more at a clock than at fixed nodes** | 512 against 256: +73.6 at fixed nodes, +44.3 timed, 11–18% slower per node | Judge width by the timed leg. The fixed-node leg tells how much knowledge the net added |
| **Wide nets want more data** | net4w512's held-out loss sat 0.5% above its training sample (net 3: −0.5%, net 4: level). Adding generation 3 (+71% data) to the same recipe gave net 5 **+21.9 ± 9.2** timed and a 9.7% shorter time to depth | Keep growing the data with each generation; a 512-neuron net plays at about 21,800 positions/s an instance, so a 600M generation costs about $9 on spot |
| **A stronger net draws more of its self-play** | Generation 3 (net4w512): 46.4% draws, against 43.0% for generation 2 (net2x80), and win adjudications 53.2% against 56.6% | Watch the draw rate as nets improve; results carry less signal as it rises, which matters for λ |
| **The ladder can't separate releases about +20–45 apart** | net 4 read 3086 ± 33 and net4w512 3076 ± 30, though net4w512 won the SPRT by +44 | The SPRT is the verdict between nets. The ladder tracks the trend over several releases |
| **At 1.5B positions, a longer schedule is saturated** | net5x120 (120 superbatches) against net 5 (80) on identical data: held-out −0.17%, SPRT +4.9 ± 3.9 timed and −1.2 ± 4.2 at fixed nodes, both inconclusive at 10,000 games. The middle LR phase flattened from superbatch 88 | Keep 80 superbatches. Revisit length only when the data grows a lot; add data or capacity instead |
| **Output buckets pay, modestly** | net5b8 (8 buckets) against net 5 on identical data: held-out −0.43%, SPRT **+6.5 ± 4.4** timed and **+12.4 ± 6.6** at fixed nodes, at +0.6% instructions a node | Keep 8 buckets. A held-out gain from capacity is worth less Elo than one from data or schedule; don't size expectations from loss alone |
| **Fail-high-first sits at its bar** | net5b8 read 0.8702 against the 0.87 bar (net 5: 0.875; net 4 forced the last lowering, 0.88 → 0.87) | Expect a net to cross it soon. Decide before that SPRT whether a crossing gets a declared exception, as EBF's did for net5x120 |
| **One ladder run is ±40, even for one net** | net6 read 3156 ± 30 (`v0.40.0`) and 3119 ± 29 (`v0.41.0`), the same net; net5b8's three releases read 3120–3127 | Never compare two single ladder runs; average several releases of a net, or use the SPRT |
| **Capacity needs the low learning rate** | net6k4's lead over net6 was −0.04% at LR 0.001 (superbatch 36) and −1.04% after both drops, still widening at 80 | For bigger nets, test a longer low-LR phase before concluding they don't pay |
| **King buckets were the best architecture step since 512** | net6k4 against net6 on identical data: held-out −1.04%, SPRT **+21.7 ± 9.0** timed and **+40.0 ± 12.4** at fixed nodes, at 10.8% slower per node | Keep 4 king buckets. A refresh cache is the cheap follow-up; a finer layout and a longer low-LR phase are the experiments |
| **A new generation's gain shrinks with the share of data it adds** | Added training data and timed SPRT: +71% gave +21.9 (net 5), +32% gave +14.5 (net6), +16% gave +6.8 (net7). Few points, and each net's recipe differs | Expect less from each generation of about this size. Test whether label quality (relabelling) pays more than volume |
| **Gains near `elo1` cost the most games** | net7 took 8,088 timed games and hit the 10,000 cap at fixed nodes; net5x120 hit the cap on both legs. Both tests use `elo1` = 5 | Budget for the cap on small changes. With both legs in the check, a real gain of about +5 can still end inconclusive |
| **Better labels on the same positions gained strength** | net7rl012 against net7: only generations 0–2's scores changed (38.6% of the records), +16.3 ± 7.7 timed and +18.1 ± 8.1 at fixed nodes (#202), against +6.8 ± 4.5 for adding generation 5 (net7). One result, one training run | Relabelling can pay as much as new data. Test relabelling more generations as its own experiment |
| **Relabelling the newer generations didn't pay** | net7rl012345 against net7rl012: only generations 3–5's scores changed, to net7rl012's (61.4% of the records), +1.9 ± 4.1 timed (inconclusive at 10,000 games) and −5.2 ± 5.2 at fixed nodes, H0 (#210). Their original labellers, net4w512 to net6k4, were much closer to net7rl012 than PeSTO to net2x80 were to net7; `N7r` agrees with the originals on sign 96.1–96.5% of the time, against 93.0–95.7% for `N7`. One result, one training run | Relabelling pays most where the labeller gains most over the original. Don't relabel generations whose labeller is close to the current net; test width and other capacity next |
| **768 wide is smarter but too slow at a clock** | net7rl012w768 against net7rl012 on identical data: held-out −1.00%, **+28.9 ± 10.2** at fixed nodes but **−9.0 ± 6.4** timed (#214), at 13.9% ± 6.9% slower per node and 20.9% longer to depth. 512 kept +44 of +74 at a clock | Width now costs more at 5+0.05 than it adds. Make the wide path cheaper (accumulator updates, a refresh cache) before going wider, and re-test this net after |

## Next experiments

One change at a time, each kept only on an accepted H1 in both the timed and
the fixed-node SPRT against the current default.

1. ~~**Longer schedule**~~: done, net2x80 (+21.0 ± 8.9).
2. ~~**λ sweep**~~: done, λ 0.5 stays (0.25: −38.1; 0.75: −3.2).
3. ~~**Generation 2 and net 3**~~: done, net 3 (+21.8 ± 9.2 against net2x80).
4. ~~**Wider hidden layer**~~: done, 256 (net 4, +74.2 ± 16.4) and 512
   (net4w512, +44.3 ± 13.1), with the output kept exact at any width.
5. ~~**Generation 3 and net 5**~~: done, net 5 (512, generations 0–3,
   +21.9 ± 9.2 against net4w512).
6. ~~**Longer schedule at 512 on generations 0–3**~~: dropped, net5x120
   (+4.9 ± 3.9 timed, −1.2 ± 4.2 at fixed nodes, both inconclusive; #159).
7. **The current loop.** Generation 4 (net 5) and generation 5 (net6k4) are
   done; `datagen.toml` is at `generation = 6`, played by net7, not started.
   - ~~**Output buckets**~~: done, net5b8 (8 buckets, +6.5 ± 4.4 timed,
     +12.4 ± 6.6 at fixed nodes; #162, #163, #164). Every net from here is
     bucketed.
   - ~~**net6:**~~ done, net5b8's recipe on generations 0–4 (+14.5 ± 7.1
     timed, +11.4 ± 6.3 at fixed nodes; #170).
   - ~~**King-bucketed inputs**~~: done, net6k4 (4 mirrored king buckets,
     +21.7 ± 9.0 timed, +40.0 ± 12.4 at fixed nodes; #172, #173, #175).
     Follow-ups: a refresh cache for speed, a longer low-LR phase, and a
     finer layout (bucket 3 holds 31% of kings over 24 squares;
     [king-buckets.md](king-buckets.md#measured)).
   - ~~**net7:**~~ done, net6k4's recipe on generations 0–5 (+6.8 ± 4.5
     timed; +5.1 ± 4.2 at fixed nodes, inconclusive at the cap; #180).
   - **Relabelling** the oldest generations with net7: the tool, the `relabel`
     profile and the workflow exist (#179, #185, #186). The first run failed on
     old manifests, fixed in #187. `N7` on generations 0–2 is complete and
     audited ([datasets.md](datasets.md#label-sets)), from
     [41788545675](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41788545675) and its top-ups, with #189 (stay on spot) and
     #196 (batches). #190 (`Train` chooses a label set) had its first real run in
     [net7rl012](#net7rl012-l050-r42054816508), which passed both SPRT legs against net7 (#202).
     Then net7rl012 relabelled generations 3–5 (`N7r`, audited;
     [datasets.md](datasets.md#net7rl012-l050-r42054816508-n5000)), the first
     label set from the relabel manager (#204).
     [net7rl012345](#net7rl012345-l050-r42398995769), `N7r` on generations 3–5, was
     **dropped**: +1.9 ± 4.1 timed (inconclusive), −5.2 ± 5.2 at fixed nodes (H0; #210).
   - **768 neurons:** [net7rl012w768](#net7rl012w768-l050-r42411319602),
     net7rl012's recipe at 768 wide, **dropped**: +28.9 ± 10.2 at fixed nodes
     (H1) but −9.0 ± 6.4 timed (H0; #214), about 14% slower per node. The engine
     reads 768 from #211. A cheaper 768 path, then a re-test of the same net, is the
     follow-up.
   - **1024 neurons or a second hidden layer:** later, at about 3B
     positions. 512 already lost 30 of its +74 fixed-node gain to speed at a
     clock, and 1024's first layer (about 1.5 MB) leaves most CPUs' L2.
8. **Retune search margins for the net's scores,** and revisit the
   improving heuristic (#95), as engine PRs.
9. **Later, if λ is revisited:** a schedule, λ rising during training.

## Adding a net

For every net trained on the GPU task, kept or not:

1. Add a row to the summary and a section with the data (generations, label
   sets, record counts, split, prepared-data SHA-256s), the schedule, the `Train` run, the timings and
   cost, the losses, the validation curve and the engine check, all from
   its `training.json`.
2. If it's tested, add the SPRT (both legs, games, the PR, and every attempt,
   including a reclaimed or re-run leg), and the gate in force when it was
   decided. If it's released, add the ladder rating of every release that
   carries it.
3. If it's dropped, say why, with the numbers. A dispatch cancelled before
   training gets a line in the net's section.
4. Update "What we've learned" when a result changes what to try next.
5. **Add the net to [datasets.md](datasets.md#which-labels-each-net-trained-on):**
   its label set per generation, from `training.json`'s `datasets` and `labels`,
   and its prepared set. Records from before #190 have no `labels`; their
   `selfplay/v1/` prefixes mean `O`.
6. **Name a labels-only change with an `rl` suffix** giving the relabelled
   generations, like net7rl012, as a schedule change takes `x`. Compare it with
   its base as in
   [net7rl012's plan](#net7rl012-l050-r42054816508).
7. Take numbers from the primary source (`training.json`, the PR's SPRT and
   bench comments, the release notes) and label anything else, such as costs,
   as an estimate. The full rule is in
   [AGENTS.md](../AGENTS.md#experiments-and-the-research-record).

## Audit log

Checks of this ledger against its primary sources.

**2026-11-03,** at `v0.44.1`:

| Check | Result |
|:--|:--|
| Both SPRT legs of all 14 tested nets (Elo, games, W / L / D) against each PR's comment | All match. net7 was missing and is now recorded |
| Ladder ratings of 22 releases, `v0.28.0`–`v0.44.1`, against the release notes | All match, except that the summary gives net 1's re-rating on the Stockfish 19 ladder (2977 ± 32, #126) where the release notes keep the original 3000 ± 37; net 1's section gives both. The re-rating itself wasn't re-derived. `v0.43.0`–`v0.44.1` were missing and are now recorded |
| Every `Train` run listed by GitHub (16) has an entry here | One was missing, cancelled run 41170474213, now under net4w512 |
| Training + held-out records of nets 1, 2, 3, 5, 6 and 7 against the sum of their generations in [datasets.md](datasets.md) | Exact, as are net7's shard total and net6's |
| net7's section against its `training.json` and `eval_check` in the run's artifact | All match; its file's SHA-256 matches `crates/search/nets/` |

**Not checked:** the `training.json` of nets other than net7, and anything in
S3, which needed AWS credentials this audit didn't have.

**2026-11-04,** at `v0.44.3`, adding the Labels column:

| Check | Result |
|:--|:--|
| The summary's training records, λ and superbatches for all 14 nets, against each `training.json` | All match |
| The label set of all 14 nets | `O`: only `data/selfplay/v1/` prefixes and no `labels` map |
| Held-out loss against each `training.json`, for the 11 nets whose summary row and record both give one | All match. Net 1 was scored afterwards (#121), and the summary leaves out the λ sweep's as not comparable |
| Every `Train` artifact downloaded | All 14 |

This closes the first item left open above. **Not checked:** anything in S3, and
`v0.44.3`'s rating, which is still running.
