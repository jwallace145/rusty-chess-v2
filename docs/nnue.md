# NNUE evaluation

The engine evaluates with an efficiently updatable neural network, trained
on its own self-play games, instead of the hand-written PeSTO tables. The net
is embedded in the binary, so an install is still one file; PeSTO stays
available for comparisons. The data the nets are trained on is described in
[datagen.md](datagen.md), and the training in [training.md](training.md).

## Architecture

```text
768 inputs ──▶ 128 hidden (our perspective)  ─┐
768 inputs ──▶ 128 hidden (their perspective) ┴─▶ SCReLU ──▶ 1 output
```

- **Inputs:** one feature per colour, piece kind and square, seen from a
  perspective. "Our" pieces are features 0–383 and the opponent's 384–767,
  each block ordered pawn, knight, bishop, rook, queen, king with 64 squares
  each. Black's perspective flips the board vertically (`square ^ 56`). This
  is `bullet`'s `Chess768` input.
- **Hidden layer:** `HIDDEN` neurons per perspective (128 to start; a constant
  in `crates/search/src/nnue.rs`, raised with the nets). The two perspectives
  share their weights.
- **Activation:** squared clipped ReLU, `clamp(x, 0, QA)²`.
- **Output:** side-to-move perspective first, then the opponent's,
  concatenated into one weighted sum and scaled to centipawns.

## Quantization

| Constant | Value | Meaning |
|:--|--:|:--|
| `QA` | 255 | hidden-layer scale: activations are clipped to `0..=QA` |
| `QB` | 64 | output-weight scale |
| `SCALE` | 400 | centipawns per unit of the network's output |

```text
eval = (Σ screlu(ours[i]) · w[i] + Σ screlu(theirs[i]) · w[H + i]) / QA + bias
eval = eval · SCALE / (QA · QB)
```

The sum is accumulated in 64-bit integers. At these scales the trainer's
clipped weights would come within a few percent of overflowing `i32`.

## File format

All values are `i16`, little-endian, in this order. A trailing pad to a
multiple of 64 bytes is allowed. Anything else is refused with the expected
size.

| Block | Count |
|:--|--:|
| feature weights, feature-major (`[768][HIDDEN]`) | 98,304 |
| feature bias | 128 |
| output weights (our perspective, then theirs) | 256 |
| output bias | 1 |

That is 197,378 bytes at `HIDDEN = 128`. It is the layout `bullet`'s simple
example saves for this architecture. **Before the first trained net is
used,** check a handful of positions against the trainer's own evaluation of
them. A mismatch means the layout or a scale is off, not that the net is
weak.

## How the engine keeps it up to date

`Board` keeps a stack of accumulators, one per ply: the hidden layer before
activation, for both perspectives.

- **`make_move`** pushes a copy and adds or subtracts the weight columns of
  the pieces that moved, were captured or promoted, or castled.
- **`unmake_move`** pops.
- **`set_root`** (the start of every search) moves the current entry to the
  bottom, so a search can go `MAX_PLY` deep whatever the game's length.
- **A long game played forward,** move by move without unmaking (UCI
  `position … moves`, self-play), restarts the stack from the bottom when it
  fills.

Nothing allocates during a search. Tests check the accumulators against a
full recomputation at every ply of random games, through castling, en
passant, promotions and captures, made and unmade.

## The embedded net

| Net | Trained on | λ | Schedule | Check against the trainer |
|:--|:--|--:|:--|:--|
| `net1-l050-r41022503872` (default) | generation 0: 214.6M positions, PeSTO self-play at 5000 nodes | 0.5 | 40 superbatches, 3.2 minutes on an NVIDIA L4 | mean 8.7 cp, max 29.8 over 256 held-out positions |

The file is `crates/search/nets/<net id>.nnue`, read with `include_bytes!`
(`nnue::default_network`). Its record (`training.json`, the losses, the
split) is in the dataset bucket under `data/nets/<net id>/`, and its SHA-256
there matches the embedded file. A new default net changes
`DEFAULT_NET_ID`, `DEFAULT_NET_GENERATION` and the file together, with a new
bench signature and a timed SPRT against the net it replaces.

## Choosing the evaluation

- **UCI:** `setoption name EvalFile value /path/to/net.nnue` loads a net
  file; `pesto` selects PeSTO; `<default>` (or an empty value) returns to
  the embedded net. A file of the wrong size is reported in an
  `info string` and ignored.
- **Bench:** `rusty-chess bench` uses the embedded net, and its signature is
  the net's. `--eval-file /path/to/net.nnue` benches another net and
  `--eval-file pesto` PeSTO, whose signature is unchanged since `v0.24.0`
  (1697503).
- **Self-play:** `datagen.toml`'s `generation` picks the player:
  generation 0 is PeSTO and the embedded net's generation (1) is the net.
  `datagen` refuses any other, and `datagen-audit` re-searches a dataset
  with the evaluation its manifests name.

## Speed

The first version is plain Rust, which the compiler vectorizes. On the bench,
a 128-neuron network searches about 20–30% fewer nodes per second than PeSTO
(with net 1: about 2.0–2.2M against 2.9M on an Apple M-series core). Explicit
SIMD through `std::arch` (AVX2 for the x86-64-v3 builds, NEON on Apple
silicon) is the next `perf` change.

## Next

1. **A pilot net:** the trainer, the preparation and the trainer-to-engine
   check are in [training.md](training.md). The first pilot nets matched the
   engine exactly, apart from quantization error (mean 9 cp).
2. **Net 1, embedded and the default**, kept on a timed SPRT win over
   PeSTO; `datagen` plays generation 1 with it.
3. **SIMD** for the accumulator updates and the output layer.
4. **Net 2** from generation 1, and the λ experiment.
