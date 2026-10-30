# NNUE evaluation

The engine can evaluate with an efficiently updatable neural network instead
of the hand-written PeSTO tables. Until the first net is trained and wins its
SPRT, PeSTO stays the default and a network is loaded only on request. The
data the nets are trained on is described in [datagen.md](datagen.md).

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

## Using a network

- **UCI:** `setoption name EvalFile value /path/to/net.nnue`. `<empty>`
  returns to PeSTO. A file of the wrong size is reported in an
  `info string` and ignored.
- **Bench:** `rusty-chess bench --eval-file /path/to/net.nnue`. This gives
  the speed with a network; the signature then belongs to that network.

## Speed

The first version is plain Rust, which the compiler vectorizes. On the bench,
a 128-neuron network searches about 19% fewer nodes per second than PeSTO
(2.7M against 3.3M on an Apple M-series core). Explicit SIMD through
`std::arch` (AVX2 for the x86-64-v3 builds) is a `perf` change for once a
trained net exists.

## Next

1. **A pilot net:** the trainer, the preparation and the trainer-to-engine
   check are in [training.md](training.md). The first pilot nets matched the
   engine exactly, apart from quantization error (mean 9 cp).
2. **Embed the winning net** with `include_bytes!`, so the binary needs no
   file, and make it the default once it beats PeSTO under SPRT.
3. **Self-play with nets:** `datagen` and the bench gain `--eval-file`, so
   generation 1 is played by the engine with net 1.
