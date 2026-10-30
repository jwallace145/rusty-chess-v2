# Runbook: self-play data generation

How to run a `Datagen` job, watch it, survive interruptions, and prove the data
is fit to train on before a net ever sees it. [datagen.md](datagen.md)
specifies the formats and the S3 layout; [nnue.md](nnue.md) covers the network
that trains on the data. The reference numbers below come from generation 0's
audit on 2026-10-30 (run `20261030-41007876197-1`, 72 shards, 18,554 games,
1,803,839 records). Compare every new dataset with them.

## The short version

```sh
# 1. Review datagen.toml (settings, and the [audit] bars).
# 2. Smoke test: 1 instance, 5 minutes, about 8M positions.
gh workflow run Datagen --ref main -f instances=1 -f minutes=5 -f note="smoke test"
# 3. The real run: up to 3 instances, at most 300 minutes each.
gh workflow run Datagen --ref main -f instances=3 -f minutes=180 -f note="gen N for net N+1"
# 4. Audit a sample, from a checkout of the data's engine_sha (printed in every manifest).
ci/datagen-sample.sh s3://jwall-engineering-rusty-chess-nnue/data/selfplay/v1/gen=N/run=RUN/ /tmp/sample
cargo run --release -p rusty-chess-tools --bin datagen-audit -- check /tmp/sample --research 2000 --bullet
# 5. Train only on a run whose audit says GO, or INVESTIGATE with every warning explained.
```

## Before a run

1. **Settings.** Everything is in `datagen.toml`, and every shard's manifest
   repeats the values it was made with.

   | Setting | Value | Why |
   |:--|--:|:--|
   | `generation` | 0 | 0 is PeSTO; raise it when the engine carries net n |
   | `nodes` | 5000 | the label's search budget: about depth 7 (p10 6, p90 9) |
   | `random_plies` | 8 | 8 or 10 random plies. **Must be even**: odd counts give white an extra random move (generation 0: black +1.4 points) |
   | `max_opening_cp` | 250 | generation 0 used 400, which let 13% of games start beyond ±200 |
   | `win_cp`, `win_plies` | 1200, 4 | generation 0 used 2000; see *Adjudication* below |
   | `draw_after_ply`, `draw_plies`, `draw_cp` | 80, 12, 10 | 6.7% of generation 0's games |
   | `max_plies` | 400 | 0.1% of games |
   | `shard_positions` | 25000 | about 1 minute per worker; the most a spot notice can cost a worker |

2. **The engine.** Data is played by `main` at the moment you dispatch.
   Start right after a release if the dataset should match a release tag.
3. **Capacity.** Each instance is a 64-core `c8a.16xlarge` on spot. The task
   budget is 192 vCPUs (3 instances), shared with SPRT and Match: three datagen
   instances make those wait.
4. **Cost and size.** About 100M positions per instance-hour, about $1.27 an
   hour on spot, and about 18 bytes per position in S3 (17.3 measured).

   | Plan | Positions | Wall time | Compute | S3 |
   |:--|--:|--:|--:|--:|
   | smoke test, 1 × 5 min | ~8M | 7 min | ~$0.15 | 0.15 GB |
   | 1 × 300 min | ~500M | 5 h | ~$6 | ~9 GB |
   | 3 × 180 min | ~900M | 3 h | ~$11 | ~16 GB |

5. **Smoke test first** after any change to `datagen`, `ci/datagen-task.sh`
   or the workflow. It must finish within seconds of its minutes, with its
   summary table filled and every shard's three files in S3.

## During a run

```sh
gh run view <run id>                                     # the three Generate jobs
aws logs tail /jwall-engineering/rusty-chess/tasks --follow --since 5m   # live progress lines
aws s3 ls s3://jwall-engineering-rusty-chess-nnue/data/selfplay/v1/gen=0/run=<run>/task=t0/manifests/ | wc -l
aws ec2 describe-instances --filters Name=tag:Purpose,Values=ci-task \
  --query 'Reservations[].Instances[].[Tags[?Key==`TaskId`]|[0].Value,State.Name,InstanceLifecycle]' --output text
```

**Healthy looks like:**
- about 28,000 positions per second per instance (`[1291s] … (28605/s)`);
- shards in S3 keeping pace with the shards the log reports;
- no `retrying` lines in the log.

## Interruptions

**What happens.** AWS gives two minutes' notice, and the runner sends
SIGTERM to the task script.
1. `datagen` runs in its own session, so only the script gets the signal.
2. The script touches the stop file.
3. Every worker finishes its game and writes its last, partial shard.
4. The script uploads every complete shard and writes `task.json` with
   `"interrupted": true`. That copy goes into the job's artifact, *not* S3.
5. The script exits 143. The runner reports `interrupted`, and the run
   summary says so.

**What is lost:** at most the games in progress, a few seconds per worker.
In a local test with the runner's signal, 82,366 of 82,366 positions were
uploaded, 17 seconds after the signal.

**How to tell afterwards:**
- the run summary's status column;
- `SPOT INTERRUPTION: <time>` in the task log;
- a task folder without `task.json` in S3.

**What to do:** nothing needs cleaning up. Every shard with a manifest is
complete and valid. To make up the time, start another run: a new run, or a
re-run of the same one, gets its own `run=` partition and new seeds.

**The first one, 2026-10-30:** t0 of `20261030-41007876197-1` was reclaimed
31 minutes in, with 49,440,814 positions safely in S3. That was before
`datagen` ran in its own session: the signal killed it outright, about 350k
positions in progress were lost, and no `task.json` was written. Both are
fixed.

## After a run: the audit

1. **Download a sample** with `ci/datagen-sample.sh RUN_PREFIX DIR
   [WORKERS_PER_TASK] [SHARDS_PER_WORKER]`. The default, 8 workers per task
   and 3 shards each, is about 1.8M positions. It is read-only and needs AWS
   credentials that can read the bucket (the owner's profile).
2. **Check out the data's `engine_sha`** if you will use `--research` or
   `determinism`, which search with the engine the audit is built from. The
   replay checks don't depend on the engine version.
3. **Run** `datagen-audit check DIR --research 2000 --bullet`. It takes about
   20 seconds for 1.8M records.

### P0: correctness. Every check must find zero, or the verdict is STOP (exit 1).

| Check | How | Generation 0 |
|:--|:--|--:|
| Records differ from the replayed game | replay every game from `start_fen` with the move generator alone, rebuild each record (position before the move, the score converted to white's view, the game's result), compare byte for byte | 0 of 1,803,839 |
| Terminations not confirmed | check the final position: mate, stalemate, repetition, fifty moves, insufficient material, ply limit; and the scores for win and draw adjudication | 0 of 18,554 |
| Record FENs that do not round-trip | parse and reprint every record | 0 |
| Checksum mismatches | each record file's SHA-256 against its manifest (shards written since checksums were added) | n/a (predates them; gzip CRC passed) |
| Other failures | illegal moves, moves after a game ended, manifest counts, seeds shared between workers, leftover lines | 0 |
| `bulletformat` (`--bullet`) | every record through bulletformat 1.8.0's parser and binary writer, with the score and result stored from the side to move's view as expected | 0 failures; 57,722,848 bytes = 1,803,839 × 32 |

A STOP means don't train. Find the cause, fix it, and regenerate the
affected run. Records are only as trustworthy as the check that failed.

### P1: quality. Bars in `[audit]` of `datagen.toml`; outside one is INVESTIGATE.

| Statistic | Bar | Generation 0 | What it guards |
|:--|:--|--:|:--|
| Draw rate | 20–55% | 34.6% | drawish or one-sided play |
| White–black win gap | ≤ 3 points | 1.4 | colour bias (odd random plies) |
| Duplicate positions | ≤ 5% | 2.2% | collapsed openings, repetition loops |
| Distinct openings | ≥ 99% | 100% | seed reuse |
| \|score\| ≤ 100 | 25–60% | 39.0% | a near-zero, draw-dominated dataset |
| \|score\| > 700 | ≤ 20% | 10.9% | decided positions crowding out the rest |
| Endgame share (≤ 20 points of non-pawn material) | ≤ 50% | 45.8% | long won endgames |
| Kept share of searched plies | 60–85% | 73.2% | the filter cutting too much or too little |
| Median game length (engine plies) | 60–250 | 125 | broken adjudication or endless shuffling |
| Fresh re-search: correlation within ±1500 (`--research`) | ≥ 0.95 | 0.989 | labels from the wrong position or perspective |
| Fresh re-search: same sign where \|label\| ≥ 50 | ≥ 98% | 99.7% | sign errors |
| Fresh re-search: median depth | ≥ 6 | 7 | a node budget too small to label |

The full correlation against a fresh search is lower (0.53–0.61). The labels'
large endgame scores, which the in-game search reaches with its table, pull it
down, and so does the odd mate a fresh search finds. The trimmed figure is the
one that tests the labels.

### Deeper checks, for a new generation or a changed setting

| Command | Answers |
|:--|:--|
| `datagen-audit outcomes DIR` | game result by score bucket, split by how games ended; the logistic scale of scores against results |
| `datagen-audit thresholds DIR` | whether lower win adjudication would have called the right winner, and how many searches it saves |
| `datagen-audit adjudication DIR 200` | deep search (2M nodes) and a playout (20k nodes a move) of win-adjudicated games |
| `datagen-audit repetition DIR 300` | deep search before each repetition's final move |
| `datagen-audit openings DIR` | what the start positions are, and whether games converge |
| `datagen-audit determinism DIR` | the first 12 games of a shard regenerated from its seed (needs the data's engine) |

## What the generation-0 checks found

### Adjudication is safe; 2000 cp was more than needed

- **At 2000 cp, every adjudicated game was already lost.** Of 200 sampled
  win-adjudicated games, a 2-million-node search (400× the game's budget)
  found a forced mate in 100%, with a median of mate in about 3 moves. Played
  out at 20,000 nodes a move, 200 of 200 ended with the adjudicated winner,
  in a median of 5 plies.
- **Lower thresholds,** applied to the same games, which played on past them:

  | Threshold (4 plies) | Right winner | Ended drawn | Reversed | Searches saved |
  |--:|--:|--:|--:|--:|
  | 600 | 98.31% | 1.69% | 0.01% | 13.5% |
  | 800 | 99.22% | 0.78% | 0 | 9.1% |
  | 1000 | 99.85% | 0.15% | 0 | 6.1% |
  | **1200** | **99.98%** | **0.02%** | **0** | **3.4%** |
  | 1500 | 99.99% | 0.01% | 0 | 1.6% |
  | 2000 | 100% | 0 | 0 | 0 |

  1200 is the setting from now on: 1 wrong result in 5,000 adjudications.
  It trims endgames only modestly. Most of the 46% endgame share comes from
  long drawn games, not from late adjudication.

### Why games end at the first repetition, and why that does not bias the results

- **The mechanism.**
  - The search scores any position that repeats one earlier in the game
    (including the history before the search's root) as exactly 0. In
    `search.rs::negamax`: `if ply > 0 && (board.is_repetition() ||
    board.is_fifty_moves()) { return 0; }`.
  - `Board::is_repetition` looks back over the plies the fifty-move counter
    spans, and one earlier occurrence is enough.
  - So when the search's best move leads into a repetition, its score is 0
    because the search sees a draw, and it plays the move.
  - `datagen::board_termination` then ends the game at once as a draw.
- **Why that is right here.**
  - Both sides are the same deterministic engine, so a position it has
    chosen to repeat would repeat a third time anyway.
  - Stopping at the first repeat saves the plies and drops the duplicate
    positions.
- **Evidence.**
  - All 3,553 repetition games in the sample ended on a score of exactly 0.
  - A 2M-node search from the position before the repeating move scored
    95.3% of them within ±50 of zero.
  - It found a better alternative (+50 or more for the side that repeated)
    in only 4.7%, and in none was the repetition a rescue for a side that
    was losing.
  - 14.6% of those games had a score of 100 or more somewhere in their last
    10 plies, so most are genuinely level when they end.
- **Effect on training.** The draw label agrees with the score in about 95% of
  these games. The 4.7% where one side could have kept a small edge are about
  0.9% of all games, scored 0.5 where a deeper engine might have won some. That
  is noise at the level of the labels' own search, not a systematic bias. The
  game records keep every repetition game whole, so a later generation can
  re-score them.

### How scores and results agree

Mean result (white's view) by score bucket (white's view), records per
category:

| Score | Checkmate | Win-adjudicated | Repetition | Draw-adjudicated | Other natural draws |
|--:|--:|--:|--:|--:|--:|
| < −700 | 0.000 | 0.000 | 0.5 | 0.5 | 0.5 |
| −300..−100 | 0.149 | 0.071 | 0.5 | 0.5 | 0.5 |
| −100..−25 | 0.381 | 0.261 | 0.5 | 0.5 | 0.5 |
| −25..25 | 0.371 | 0.492 | 0.5 | 0.5 | 0.5 |
| 25..100 | 0.662 | 0.723 | 0.5 | 0.5 | 0.5 |
| 100..300 | 0.878 | 0.929 | 0.5 | 0.5 | 0.5 |
| > 700 | 1.000 | 1.000 | 0.5 | 0.5 | 0.5 |

**Read it with care.** The categories are defined by how the game ended, so
every draw column is 0.5 by construction, and the decisive columns are 0 or 1.
The comparison that matters is between games that ended in checkmate and those
adjudicated as won. Their curves agree closely (+100..300: 0.878 against
0.929), so adjudication is not distorting results.

Over all records, results follow `1 / (1 + e^(−score/K))` best at **K ≈ 200**
(cross-entropy 0.51). That is the scale at which scores and results speak the
same language, and the training experiment below starts from it.

### Openings are diverse in the ways that matter

- **All different:** 18,554 games had 18,554 distinct start positions and
  17,968 distinct pawn structures (96.8%).
- **Still recognisably chess:** 89.3% have full material, 10.4% a material
  imbalance, 35.4% reduced castling rights, 12% a king off its home rank, and
  23.6% a queen off its home rank.
- **They don't converge.** After 10, 20 and 40 engine plies, 99.9%, 99.7%
  and 97.9% of games were still in distinct positions.
- **They don't collapse into one opening.** The engine's first moves were 839
  different moves; the most common, d2d4, came in 4.4% of games.
- **Not all sensible:** random plies produce some odd positions (a king
  walk, an early queen sortie). The `max_opening_cp` check discards the
  lopsided ones, and the engine's own play takes over from the first
  searched move.

## Preparing the data for training

- **Splits.** Keep every game in one split: assign whole *workers*
  (`<task>-w<NNN>`) to train, validation and test, never individual records.
  A game never spans shards, and a worker's shards come from one seed.
  Cross-game duplicates are 2.2%; remove validation positions that also occur
  in training, by the FEN's first four fields, for clean metrics.
- **Conversion.** `bullet-utils convert --from text` reads the records as
  written; `datagen-audit --bullet` runs that exact parser. The binary format
  keeps pieces, side to move, score and result, and drops castling rights,
  en passant and the move counters, which the 768-input network doesn't use.
- **Shuffling.** Records are written game by game, so adjacent records are
  correlated. Shuffle globally (bullet's loader does) or interleave shards.

## The training experiment: score against result

How much the net should learn from the search's score versus the game's
result is the first decision for net 1, and it should be measured, not
guessed. `bullet` trains on `target = (1 − λ) · sigmoid(score / K) + λ · result`.

- **Fixed:**
  - the architecture (768 → 128 × 2 → 1, SCReLU, `QA` 255, `QB` 64,
    `SCALE` 400);
  - generation 0's data, with the same train, validation and test workers;
  - optimiser, learning-rate schedule, batch size, number of epochs, and
    seed;
  - `K` = 200 (fitted above). One control run at bullet's usual 400.
- **Varied:** λ ∈ {0.0 (score only), 0.25, 0.5, 0.75, 1.0 (result only)}.
  That's five nets, plus the K = 400 control at the best λ.
- **Measured, in this order:**
  1. **Strength:** each net against PeSTO in an SPRT (timed, the gate as
     always), then the best two or three against each other with the
     `Match` workflow at 10+0.1 and a 99% interval. **Strength decides.**
  2. **Validation losses,** against scores and against results, on the
     held-out workers.
  3. **Calibration:** the net's evaluation by bucket against results on the
     validation set, as in the table above.
- **Expectation:** with depth-7 labels and an engine's first generation,
  λ ≈ 0.25–0.5 usually beats both pure score and pure result. Pure result is
  too noisy, and pure score copies the label search's mistakes. Later
  generations, with better labels, tend to lower λ.

## Troubleshooting

| Symptom | Likely cause | Action |
|:--|:--|:--|
| Plan job fails: no dataset component | `jwall-engineering-sites.yaml` lacks the `dataset` component | add it (see [datagen.md](datagen.md)) |
| Plan job fails: minutes | over 300: the 6-hour runner limit also covers slot waits, boot, build and upload | at most 300 |
| A Generate job waits, then fails | no task slot: SPRT, Match or other datagen instances fill the 192 vCPUs | wait, or run fewer instances |
| Interrupted | spot reclamation | start another run; the data is valid |
| No capacity | spot unavailable in every zone, and on-demand fallback off or failed | re-run later |
| Shards in S3 lag the log | uploads behind | look for `retrying` in the log; batched uploads keep about 4× ahead of 64 workers |
| Audit STOP | a correctness failure | don't train; the failing lines name the shard and record |
| Audit INVESTIGATE | a quality bar | explain or fix before training; record the explanation with the net |
