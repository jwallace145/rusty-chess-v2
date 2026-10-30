# NNUE nets

The ledger of every net trained, kept or not: what it was trained on and
how, what it measured, and how it played. It's the record for deciding what
to try next, so it keeps the failures too. The data each net names is in
[datasets.md](datasets.md); the architecture and file format in
[nnue.md](nnue.md); the training pipeline in [training.md](training.md).

Each net's own record (`training.json`, the losses, the validation curve, the
split, the log) is in `s3://jwall-engineering-rusty-chess-nnue/data/nets/<net id>/`.
Net ids are `<name>-l<λ×100>-r<Train run id>`.

## Summary

| Net | Status | Data (generations, training positions) | Architecture | λ | Superbatches | Held-out loss | Engine check (mean / max cp) | Timed SPRT vs previous | Ladder (Stockfish 19) |
|:--|:--|:--|:--|--:|--:|--:|:--|:--|:--|
| `net1-l050-r41022503872` | Default `v0.28.0`–; embedded | 0 · 214.6M | (768 → 128) × 2 → 1 | 0.5 | 40 | 0.018925 (gen-0 held-out) | 8.72 / 29.82 | **+587.3 ± 82.7** vs PeSTO | 2977 ± 32 (`v0.28.0`) |
| `net2-l050-r41110004652` | Default in `v0.30.0`; replaced by net2x80 if its SPRT passes | 0 + 1 · 561.7M | (768 → 128) × 2 → 1 | 0.5 | 40 | 0.019273 (gen-0 + gen-1 held-out) | 5.52 / 20.32 | **+112.8 ± 21.1** vs net 1 | after its release |
| `net2x80-l050-r41131366975` | Candidate (#134): timed SPRT against net 2 | 0 + 1 · 561.7M | (768 → 128) × 2 → 1 | 0.5 | **80** | **0.019204** (same held-out set as net 2: 0.019273) | 5.67 / 22.38 | pending | — |

**Comparing the columns:**
- **Held-out loss only compares on the same held-out data.** Net 2's
  includes generation 1, which has more draws and different labels, so its
  0.019273 is not worse than net 1's 0.018925. Score two nets on one set
  with `rusty-chess-trainer validate` to compare them.
- **The timed SPRT is the verdict.** It's self-play against the previous
  net, so it overstates gains against other engines. The ladder is the
  external scale.

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
| Ladder | After its release |

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
| SPRT vs net 2 | Pending (#134) |

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
| **Held-out loss doesn't compare across datasets** | Net 2's 0.019273 against net 1's 0.018925, on different held-out data, while net 2 is +113 stronger | Compare nets on one held-out set |

## Next experiments

One change at a time, each kept only on a timed SPRT win over the current
default.

1. **Longer schedule:** net 2's data at 80 superbatches.
2. **λ sweep:** λ ∈ {0, 0.25, 0.75, 1} on generations 0 + 1. It's cheap:
   one GPU at a time, about 40 minutes a net. The best candidates go to a
   `Match` or an SPRT.
3. **Generation 2:** played by net 2 once #129 is released, then net 3 on
   generations 0–2.
4. **Wider hidden layer:** 256, then 512. These need the engine's `HIDDEN`
   to change, cost speed per node, and want more data. Measure them on the
   largest dataset available.
5. **Retune search margins for the net's scores,** and revisit the
   improving heuristic (#95), as engine PRs.

## Adding a net

For every net trained on the GPU task, kept or not:

1. Add a row to the summary and a section with the data (generations,
   record counts, split), the schedule, the `Train` run, the timings and
   cost, the losses, the validation curve and the engine check, all from
   its `training.json`.
2. If it's tested, add the SPRT (both legs, games, the PR). If it's
   released, add the ladder rating.
3. If it's dropped, say why, with the numbers.
4. Update "What we've learned" when a result changes what to try next.
