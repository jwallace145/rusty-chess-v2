# King-bucketed inputs: design

A design note for the next NNUE architecture change: giving each side's
input layer a few sets of weights, chosen by where that side's own king
stands. It records the decisions that are hard to reverse (the bucket
layout, the feature indexing, the file format) and the plan for building
and testing them. The architecture as it stands is in [nnue.md](nnue.md);
the experiments that led here are in [nets.md](nets.md).

## Why

Today every piece-square feature has one weight column, whatever the
position. "Knight on f6" means the same to the net with the king castled on
g1 as with it stuck on e1. King safety, the main thing a hand-written
evaluation adds terms for, can only be learned indirectly.

With **king buckets**, each side's 768 features get a separate set of
weights for each region of the board its own king can stand in, so a
piece's value depends on where the king it attacks or defends is. It is the
standard way an NNUE learns king safety: no hand-made features, which would
break the incremental update (a move changes 2–4 features; "attackers near
the king" can change after any move).

**Horizontal mirroring** halves the regions needed: when a side's king is
on files e–h, that side sees the board mirrored left to right, so a king on
g1 and one on b1 share weights. Positions are mostly symmetric that way;
castling rights are not features.

Other engines report this as the largest architecture gain after output
buckets; it is the next experiment in [nets.md](nets.md#next-experiments).
Our SPRT decides whether it stays.

## The bucket layout (fixed in the format)

Four buckets over the half-board a mirrored king can stand in. From each
side's own point of view (rank 1 is its back rank), files a–d:

```text
rank 8   3 3 3 3
  ...    3 3 3 3      (ranks 3–8: the king has left home; mostly endgames)
rank 2   2 2 2 2
rank 1   0 0 1 1      (a, b: castled; c, d: central or queenside)
         a b c d      (files e–h mirror these: h, g → 0; f, e → 1)
```

- **Why these four:** most positions have the king on its back rank, so it
  gets two buckets: castled into the corner (g1/h1, or b1/a1 after
  mirroring) and central (d1/e1, c1/f1). A king one step forward is a
  distinct, common middlegame shape. Everything further up is mostly
  endgame, where king safety matters least and data is thinner.
- **Why not more:** each bucket is another 768 × H weights to train. At
  about 2B positions (generations 0–4), four keeps every bucket well fed;
  the back-rank buckets see most of the data. More buckets can come with
  more data, as a new layout.
- **The layout is part of the file format.** A net file records its bucket
  *count* through its size (below), not the layout, so a count always means
  one layout: **4 means the table above, forever**. A different layout gets
  a different count or a header. The constant lives in
  `crates/search/src/nnue.rs` and the trainer, with a test that they match.

## Feature indexing (must match bullet exactly)

bullet's `ChessBucketsMirrored` (at the pinned commit) defines the feature
for one perspective:

```text
index = 768 × bucket(k) + (chess768 index of the piece, from this side's view) XOR flip
  k     = this side's own king square, from its own side of the board
  flip  = 7 if k is on files e–h, else 0
```

- **XOR 7 mirrors the file:** a chess768 index is `384·theirs + 64·kind +
  square`, and the blocks start at multiples of 64, so XOR 7 changes only
  the square's file (a ↔ h, b ↔ g, ...).
- **Each side uses its own king,** seen from its own side: white's
  perspective uses white's king square, black's uses black's king square
  flipped vertically (`^ 56`), which is how bulletformat stores
  `opp_ksq`. The engine's existing perspective features already orient
  squares this way.
- **One bucket is today's indexing:** bucket 0, no flip. Every existing net
  evaluates exactly as before, and every bench signature is unchanged.

## The accumulator

A side's accumulator depends on its own king's bucket and mirror ("its
view"). A side's view changes only when **its own king** moves.

- **Most moves:** both views unchanged; the incremental update is today's,
  with each side's feature indices computed in its view.
- **A king move that keeps its view** (e.g. g1–h1): incremental, as now.
- **A king move that changes its view** (crossing between files d and e, or
  between buckets): that side's accumulator is **rebuilt** from the
  position after the move (the bias plus about 32 columns, against 2–4).
  The other side's is updated incrementally as usual.
- **Unmake** pops the stack as now: the previous ply's accumulators are
  kept, so nothing is recomputed going back.

**No refresh cache in the first version.** Strong engines keep a per-view
cache of accumulators (a "Finny table") so a rebuild only applies the
difference from the last position seen in that view. View changes are rare
in search (castling, and king walks in endgames), so the first version
rebuilds from scratch and the paired bench measures the cost. The cache
follows as a `perf` PR, with the signature unchanged, if the cost shows.

**It showed.** A profile at `v0.46.0` found 8.4% of pushes rebuilding a
perspective, 8.1% of search time at 512 and 11.3% at 768: the layout's
boundaries (files d/e, ranks 1/2, f1/g1) are where kings step. The cache is
planned in [nnue.md](nnue.md#planned-a-refresh-cache-for-king-bucket-rebuilds).

`make_move` already pushes the accumulators after the position has made the
move, so the rebuild has the position it needs; the move delta also carries
the king's from and to squares.

## File format

Unchanged except the feature-weight block grows:

| Block | Count |
|:--|--:|
| feature weights, feature-major (`[768 × K][H]`, bucket 0's 768 rows first) | 768 · K · H |
| feature bias | H |
| output weights, per output bucket (ours, then theirs) | 2H · B |
| output biases | B |

- **K, the king-bucket count, is 1 or 4** (`nnue::KING_BUCKETS`), told apart
  by size like the width H and the output buckets B. Every (H, B, K) the
  engine reads is thousands of bytes from every other.
- **Size:** a 512-wide, 8-output-bucket net goes from 803,856 bytes to
  3,163,152 at K = 4 (the input weights are 98% of it). Only the columns of
  the active bucket are read during search, so the working set per position
  is the same as today's.
- **The factoriser is not in the file** (below): the engine sees plain
  per-bucket weights.

## Training

- **Inputs:** `ChessBucketsMirrored::new(LAYOUT)` instead of `Chess768`,
  with the layout above (32 entries, files a–d).
- **Factoriser:** a shared 768 × H weight block (`l0f`) trained alongside the
  buckets and added to every bucket's weights. A bucket with little data
  still learns the general piece-square values from all positions. It is
  bullet's own example (`3_input_buckets.rs`).
- **Clipping:** `l0w` and `l0f` are each clipped to ±0.99, so their sum stays
  within the ±1.98 the quantisation assumes.
- **Saving:** `net.nnue` gets `l0w + l0f` (the factoriser added to every
  bucket), quantised as today.
- **The float scorer needs the factoriser.** bullet's `raw.bin` skips save
  transforms, so it would hold `l0w` without `l0f`. The trainer lists `l0f`
  **last** in the save format: bullet then writes it at the end of `raw.bin`
  (the scorer adds it back) and of `quantised.bin`, which the trainer
  truncates to the engine's size. The scorer-against-bullet check (≤ 0.5 cp)
  proves the reconstruction, as it did for output buckets.
- **Data:** no new data. Every record stores both kings' squares, so
  generations 0–4 train it as they are.
- **Built as designed** in #173: the scorer reconstructs the factoriser from
  `raw.bin` to 0.0012 cp of bullet's evaluation on net6k4.

## Measured

**Where the kings are.** Each side's king in 2,000,000 of generation 4's
held-out positions (both sides counted), by bucket:

| Bucket | Squares (own side, files e–h mirrored) | Share of kings |
|:--|:--|--:|
| 0 | a1, b1 (castled into the corner) | 23.3% |
| 1 | c1, d1 (central back rank) | 23.7% |
| 2 | rank 2 | 22.1% |
| 3 | ranks 3–8 | 30.9% |

- **Balanced:** every bucket gets 22–31% of the data, about 400–600M of
  generations 0–4's 1.9B training positions, so none starves. Fewer than half
  the kings are on the back rank: the data is about 46% endgame positions.
- **Room for a finer layout:** bucket 3 holds 31% of the kings over 24
  squares, from an advanced middlegame king to an endgame king on rank 7. A
  later layout could split it (say ranks 3–4 and 5–8). It would be a new
  bucket count, since 4 always means the table above.

**The first king-bucketed net, net6k4** (full record in
[nets.md](nets.md#net6k4-l050-r41282883885)):
- **Training cost:** 3,891 s to trained against net6's 3,768 (+3%).
- **Held-out loss:** −1.04% against net6 on identical positions, all of it
  after the first learning-rate drop (−0.04% at superbatch 36).
- **Speed:** 10.8% ± 2.3% slower per node and 6.8% ± 2.9% longer to depth
  than net6 on the runner (4,143 instructions a node against 3,799), with EBF
  1.94 → 1.86 and fail-high-first 87.3% → 87.9%.
- **The SPRT (#175): kept.** +21.7 ± 9.0 timed and +40.0 ± 12.4 at fixed
  nodes against net6; `v0.42.0` rated 3166 ± 31 on the ladder.

## Plan

One change per PR, each verified before the next:

1. **Engine (`feat(nnue)`):** `KING_BUCKETS = [1, 4]`, the layout constant,
   view-dependent indexing, the rebuild on a view change, size detection,
   `docs/nnue.md`. Tests: make and unmake against a fresh computation
   through king walks that cross every bucket and the mirror line; a
   horizontally mirrored position evaluates identically under a 4-bucket
   net; every embedded net's signature unchanged; exactness in every shape.
   No SPRT: the signature is unchanged, and the paired bench measures speed.
2. **Trainer (`feat(trainer)`):** `--king-buckets 1|4`, the factoriser and
   clipping, `l0f` last with the truncation, the scorer's reconstruction,
   `train-task.sh` and the `Train` workflow input, `docs/training.md`.
   Verified end to end on a small local training run: the scorer against
   bullet, and the engine check on the saved net through step 1's engine.
3. **Train `net6k4`:** net6's recipe (generations 0–4, 512 hidden, 8 output
   buckets, λ 0.5, 80 superbatches, the same seeds) with 4 king buckets.
   The prepared data is byte-identical to net6's, so held-out losses compare
   directly.
4. **Embed and SPRT against net6.** Kept only on a timed H1, as every net.

## What could go wrong

| Risk | Guard |
|:--|:--|
| The engine's indexing differs from bullet's (orientation, mirror, bucket table) | The engine check on a trained net (a mismatch reads in the hundreds of cp), and a test that the engine's and the trainer's layout constants agree |
| A missed rebuild leaves a stale accumulator after a king move | The make-and-unmake test against a fresh computation at every ply, with walks through every view change |
| Rebuilds make search slower | The paired bench in step 1 (old nets never rebuild, so it is the indexing cost) and the timed SPRT in step 4; the refresh cache if needed |
| Too little data per bucket | The factoriser; four buckets, not more; held-out loss against net6 on identical positions before the SPRT |
| The 3 MB net is slow to load or bloats the binary | Measured in step 4; the binary embeds one net per generation, so it grows by about 3 MB |
