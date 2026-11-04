# Relabelling self-play data: design

A design note for re-scoring stored self-play positions with a stronger
engine, keeping every original label. It records the decisions that are hard
to reverse (where new labels live, what a relabelled shard contains, how a
position is re-scored) and the plan. How the data is made is in
[datagen.md](datagen.md); the generations are in [datasets.md](datasets.md).

## Why

Every training record's target is `λ · result + (1 − λ) · sigmoid(score /
400)`, and the score is frozen when the generation is played: generation 0's
positions carry PeSTO's 5,000-node scores, generation 1's net 1's, and so on.
About a third of today's data is labelled by PeSTO or net 1. The λ sweep is a
hint that label quality matters: weighting the 5,000-node scores more (λ
0.25) lost 38 Elo.

Relabelling replays each stored game and re-scores its kept positions with a
stronger net (net6k4 today). Training net7's recipe on relabelled data and
SPRT-testing it against net7 isolates label quality: same positions, same
split, same results, new scores.

## What stays and what changes

- **The positions stay.** A relabelled shard has exactly the original's
  records, in the same order, with the same FEN and the same game result. Only
  the score changes. So the record count matches the manifest, `prepare`
  splits by the same run, task and worker keys, and a net trained on
  relabelled data sees the same positions in the same held-out set.
- **No position is dropped.** The original keep filter (not in check, quiet
  best move, no mate score) chose these positions with the old engine's
  search. A new search may find a mate there: the score is then clamped to
  ±3,000 cp (the target saturates long before), and the count is recorded in
  the manifest. Dropping positions would change the split's record counts
  and the comparison.
- **The originals are never touched.** New labels go to a separate prefix;
  the dataset bucket is append-only.

## How a position is re-scored

For each game in a shard, in order:

1. start from the game's `start_fen` with the relabelling net, the
   transposition table cleared (datagen clears it at the start of each game);
2. at every ply, search the position at `nodes` (5,000 by default, datagen's
   budget), as datagen did, so the table and the search history evolve as
   they did in play;
3. when the position is the next record of the shard (same FEN, counters
   included), take the search's score, from white's point of view, as its new
   label;
4. play the game's **recorded** move, not the new search's choice, and go on.

Every record must be matched, in order, by the end of the shard; anything
else is an error, never a silently shorter file. Searching every ply costs
about a third more than searching only the kept positions, but keeps the
search's context the same as in play.

**Not bit-exact with the original labels, even with the same net.** Datagen
chose each opening by searches from the cleared table, and those searches
left the table and the history tables in a state the stored game doesn't
record; every later search in the game inherited it. Relabelling starts each
game cleared, so its searches see different state.

**Measured** on 16 generation-5 shards (400,762 records) relabelled with
net6k4 at 5,000 nodes, the net and budget that played them:

| | |
|:--|:--|
| Records matched, FEN and result identical, in order | all 400,762 |
| Same output from 16 threads and from 5 | 16 of 16 shards byte-identical |
| Sign agreement with the original labels | 97.7% |
| Correlation with the original labels | 0.990 |
| Scores changed | 91.4% |
| Change: median / 90th percentile / mean | 17 / 70 / 30 cp, against a mean \|score\| of 291 |
| Mate scores clamped | 16 |

- **A 5,000-node score carries about 17–30 cp of noise from search state
  alone.** That is the floor for reading a relabel: changes from a stronger
  net well beyond it are the new net's opinion.
- **The verification is therefore structural:** every record matched in
  order (a shifted record, a wrong FEN or a game mismatch is an error),
  deterministic output, and sign agreement and correlation that a wrong
  point of view could not produce.
- **Speed:** about 250 records a second a core on an Apple M-series core,
  close to datagen's rate.

## Where the labels live

```text
s3://<dataset bucket>/data/selfplay/relabel/<label id>/
  gen=N/run=R/task=T/
    positions/<shard>.txt.gz      the original records with new scores
    manifests/<shard>.json        the original manifest plus a `relabel` block
```

- **The label id** names the engine and budget: `<net id>-n<nodes>`, for
  example `net6k4-l050-r41282883885-n5000`. One id is one labelling, forever.
- **The same partition structure** as `selfplay/v1/gen=N/`, so training on
  relabelled data is the same download with a different prefix:
  `--dataset s3://…/selfplay/relabel/<label id>/gen=N/`.
- **Under `selfplay/`:** the GPU training profile can already read it.
- **No games file:** the games are unchanged; the manifest points at the
  original shard.

The manifest is the original's with a `relabel` block added: the label id,
the net's id and SHA-256, nodes, the engine version and commit, the source
positions file's SHA-256, the new file's SHA-256, and statistics (records,
scores changed, mean and largest change, mate scores clamped). Its
`positions` count and provenance are unchanged. Its `sha256` holds the new
positions file and the unchanged games file, including for generation 0, whose
original manifests predate checksums.

## Running it

- **The tool:** `relabel` in `crates/tools`, one worker per core, a shard at
  a time, deterministic for a given net, budget and shard.
- **Which net:** `--net` takes the embedded (default) net's id, or a
  retired net's id with `--net-file`, its file from the dataset bucket
  (`data/nets/<id>/net.nnue`), which must match the SHA-256 in
  `nnue::RETIRED_NETS`. The binary embeds only the default net
  ([nnue.md](nnue.md#the-embedded-net-and-the-retired-ones)).
- **In the cloud:** the `Relabel` workflow runs `ci/relabel-task.sh` on up to
  three instances of the `relabel` task profile (`c8a.16xlarge`, spot): it may
  read `selfplay/` and write only `selfplay/relabel/`, so it can never change
  an original. It always uses the commit's default net, since the profile
  can't read `nets/`. It never falls back to on demand: it retries spot across
  the zones for 30 minutes, then the task fails with `no-capacity` without
  starting an instance, and running the workflow again is free.

  ```sh
  gh workflow run Relabel --ref main -f generations=0,1,2 -f instances=3 -f minutes=280
  ```
- **Cost:** about datagen's speed, since it searches every ply: about 19,500
  records a second an instance with net6k4. Generations 0–2 (906M records)
  are about 13 instance-hours, roughly $15 on spot; all six generations (1.85B)
  about 26, roughly $30. Estimates from generation 5's measured rate. They are
  low when a worker has fewer shards than the instance has cores, because
  workers run one at a time: generation 0 (320 workers, at most 53 shards each)
  is about 7.3 instance-hours rather than 3.3.
- **The unit of work** is one self-play worker's shards in one task. Each of
  the N tasks takes every Nth worker, relabels it, and uploads each shard's
  positions and then its manifest, so a manifest in S3 means a whole shard.
- **Resumable:** a task skips shards whose relabelled manifest already
  exists, so a spot interruption only costs the shards in progress. Running
  the workflow again with the same `instances` finishes what is left.

## Plan

1. **The tool and this note** (`feat(tools)`): `relabel`, its tests, and the
   local verification above.
2. **The cloud run:** the `relabel` profile (jwall-engineering-infra
   `v0.18.0`), `ci/relabel-task.sh` and the `Relabel` workflow.
3. **Training on it:** the `Train` workflow's choice of label set per
   generation, a follow-up: a relabelled generation is the same download from
   `selfplay/relabel/<label id>/gen=N/`, which the GPU profile can already
   read.
4. **The experiment:** relabel generations 0–2 (the three weakest labellers)
   or all six with the default net, train the default net's recipe on them,
   SPRT against it.
