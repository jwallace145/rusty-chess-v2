//! NNUE evaluation: a (768 → H) × 2 → 1 network with squared clipped ReLU,
//! quantized to `i16`, evaluated from an accumulator that make and unmake
//! update incrementally. H is 128, 256 or 512 ([`WIDTHS`]), read from the file's
//! size; each width's code is compiled separately ([`Net`], [`Acc`]) and
//! chosen once per call ([`Network`], `NnueState`).
//!
//! **Inputs.** 768 features per perspective: one for each colour, piece kind
//! and square. From a perspective, "our" pieces are features 0–383 and the
//! opponent's 384–767, each block ordered pawn, knight, bishop, rook, queen,
//! king, 64 squares each; black's perspective flips the board vertically
//! (square ^ 56). This is `bullet`'s `Chess768` layout.
//!
//! **Hidden layer.** One accumulator of H `i16` values per
//! perspective: the feature bias plus the weight column of every active
//! feature. A move only adds and subtracts a few columns.
//!
//! **Output.** For the side to move, `screlu(ours) · w[..H] +
//! screlu(theirs) · w[H..]`, scaled back by `QA`, plus the output bias, times
//! [`SCALE`] over `QA · QB`, in centipawns. `screlu(x) = clamp(x, 0, QA)²`.
//!
//! **File format** (all `i16`, little-endian, in this order; a trailing pad
//! to a multiple of 64 bytes is allowed): feature weights `[768][H]`,
//! feature bias `[H]`, output weights `[2 * H]` (ours, then
//! theirs), output bias `[1]`. This is what `bullet`'s simple example saves
//! for this architecture at `QA = 255`, `QB = 64`, `SCALE = 400`; the first
//! trained net is checked against the trainer's own evaluation of a few
//! positions before it is used (see `docs/nnue.md`).

use std::sync::{Arc, OnceLock};

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind, Square};

/// A net embedded in the binary.
pub struct EmbeddedNet {
    /// Its id: `data/nets/<id>/` in the dataset bucket holds its
    /// `training.json`, and `docs/nets.md` its history.
    pub id: &'static str,
    /// The self-play generation it plays: `datagen` at this generation
    /// evaluates with it (generation 0 is PeSTO).
    pub generation: u32,
    bytes: &'static [u8],
}

impl EmbeddedNet {
    /// The net's file as embedded, byte for byte: its SHA-256 is the one in
    /// the net's `training.json`.
    pub fn file_bytes(&self) -> &'static [u8] {
        self.bytes
    }
}

/// Every net the binary carries, oldest first; the last is the default.
/// Older nets stay so the data they played can still be re-searched
/// (`datagen-audit`) and so they can be selected by id for comparisons. A net
/// is about 200 KB at 128 hidden neurons, 400 KB at 256 and 790 KB at 512.
pub const NETS: [EmbeddedNet; 3] = [
    EmbeddedNet {
        id: "net1-l050-r41022503872",
        generation: 1,
        bytes: include_bytes!("../nets/net1-l050-r41022503872.nnue"),
    },
    EmbeddedNet {
        id: "net2x80-l050-r41131366975",
        generation: 2,
        bytes: include_bytes!("../nets/net2x80-l050-r41131366975.nnue"),
    },
    EmbeddedNet {
        id: "net4w512-l050-r41170687764",
        generation: 3,
        bytes: include_bytes!("../nets/net4w512-l050-r41170687764.nnue"),
    },
];

/// The net the engine evaluates with unless told otherwise.
pub const DEFAULT_NET_ID: &str = NETS[NETS.len() - 1].id;

/// The self-play generation the default net plays.
pub const DEFAULT_NET_GENERATION: u32 = NETS[NETS.len() - 1].generation;

/// The embedded net at `NETS[i]`, parsed once.
fn embedded(i: usize) -> Arc<Network> {
    static CACHE: [OnceLock<Arc<Network>>; NETS.len()] = [const { OnceLock::new() }; NETS.len()];
    CACHE[i]
        .get_or_init(|| {
            Arc::new(Network::from_bytes(NETS[i].bytes).expect("an embedded net is well formed"))
        })
        .clone()
}

/// The default net.
pub fn default_network() -> Arc<Network> {
    embedded(NETS.len() - 1)
}

/// The embedded net that plays `generation`, if the binary carries one.
pub fn network_for_generation(generation: u32) -> Option<Arc<Network>> {
    NETS.iter()
        .position(|n| n.generation == generation)
        .map(embedded)
}

/// The embedded net with this id, if the binary carries one.
pub fn network_by_id(id: &str) -> Option<Arc<Network>> {
    NETS.iter().position(|n| n.id == id).map(embedded)
}

/// The width of the first nets (and of `Accumulator`, `FILE_BYTES` and
/// `Network::from_parts`, kept for tests and tools at that width).
pub const HIDDEN: usize = 128;

/// The hidden-layer widths the engine reads, told apart by file size.
pub const WIDTHS: [usize; 3] = [128, 256, 512];

/// Input features per perspective.
pub const INPUTS: usize = 768;

/// Quantization of the hidden layer: activations are clipped to `0..=QA`.
pub const QA: i32 = 255;

/// Quantization of the output weights.
pub const QB: i32 = 64;

/// The output in centipawns per unit of the network's raw output.
pub const SCALE: i32 = 400;

/// The largest output weight a net may have, in either sign: the trainer
/// clips weights to ±1.98, which is ±127 at `QB = 64`. It keeps the output
/// exact in 16- and 32-bit arithmetic (see [`Net::evaluate`]).
pub const MAX_OUTPUT_WEIGHT: i16 = 127;

/// The size in bytes of a network file of width `hidden`, without padding.
pub const fn file_bytes(hidden: usize) -> usize {
    2 * (INPUTS * hidden + hidden + 2 * hidden + 1)
}

/// The size in bytes of a 128-neuron network file without padding.
pub const FILE_BYTES: usize = file_bytes(HIDDEN);

/// The longest run of output terms summed in `i32` (see [`Net::evaluate`]).
const CHUNK: usize = 256;

/// A network's quantized weights at width `H`.
pub struct Net<const H: usize> {
    /// `[INPUTS][H]`, feature-major: one column per feature.
    feature_weights: Box<[[i16; H]; INPUTS]>,
    feature_bias: [i16; H],
    /// The side to move's half of the output weights.
    output_us: [i16; H],
    /// The other side's half.
    output_them: [i16; H],
    output_bias: i16,
}

impl<const H: usize> Net<H> {
    /// Widths are multiples of 16 (SIMD); `evaluate` stays exact at any of
    /// them by summing in chunks of `CHUNK`.
    const VALID: () = assert!(H % 16 == 0 && H > 0);

    fn from_words(next: &mut impl FnMut() -> i16) -> Result<Net<H>, String> {
        let () = Self::VALID;
        let mut feature_weights: Box<[[i16; H]; INPUTS]> = vec![[0i16; H]; INPUTS]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("the vector has INPUTS rows"));
        for row in feature_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = next();
            }
        }
        let mut feature_bias = [0i16; H];
        feature_bias.iter_mut().for_each(|b| *b = next());
        let mut output_us = [0i16; H];
        output_us.iter_mut().for_each(|w| *w = next());
        let mut output_them = [0i16; H];
        output_them.iter_mut().for_each(|w| *w = next());
        let output_bias = next();
        if let Some(w) = output_us
            .iter()
            .chain(&output_them)
            .find(|w| w.abs() > MAX_OUTPUT_WEIGHT)
        {
            return Err(format!(
                "an output weight is {w}, beyond ±{MAX_OUTPUT_WEIGHT}; this is not a net the trainer saved"
            ));
        }
        Ok(Net {
            feature_weights,
            feature_bias,
            output_us,
            output_them,
            output_bias,
        })
    }

    fn put_words(&self, out: &mut Vec<u8>) {
        let mut put = |w: i16| out.extend_from_slice(&w.to_le_bytes());
        self.feature_weights.iter().flatten().for_each(|&w| put(w));
        self.feature_bias.iter().for_each(|&w| put(w));
        self.output_us.iter().for_each(|&w| put(w));
        self.output_them.iter().for_each(|&w| put(w));
        put(self.output_bias);
    }

    /// A network from explicit weights (for tests and tools).
    pub fn from_parts(
        feature_weights: Box<[[i16; H]; INPUTS]>,
        feature_bias: [i16; H],
        output_us: [i16; H],
        output_them: [i16; H],
        output_bias: i16,
    ) -> Net<H> {
        let () = Self::VALID;
        assert!(
            output_us
                .iter()
                .chain(&output_them)
                .all(|w| w.abs() <= MAX_OUTPUT_WEIGHT),
            "output weights must be within ±{MAX_OUTPUT_WEIGHT}"
        );
        Net {
            feature_weights,
            feature_bias,
            output_us,
            output_them,
            output_bias,
        }
    }

    /// The evaluation in centipawns for the side to move, from the
    /// accumulators of the side to move (`us`) and its opponent (`them`).
    #[inline]
    pub fn evaluate(&self, us: &Acc<H>, them: &Acc<H>) -> i32 {
        // screlu(x) · w computed as (c · w) · c with c = clamp(x, 0, QA). With
        // every output weight within ±MAX_OUTPUT_WEIGHT (enforced at load),
        // c · w fits an i16 (255 · 127 = 32,385), and a run of CHUNK terms an
        // i32 (255² · 127 · 256 = 2,114,092,800). Each side is summed in i32
        // a chunk at a time and the chunks are added in i64, so this is exact
        // at any width: the same result as summing in i64. At 128 and 256
        // there is one chunk a side, as before 512 existed. In this shape the
        // compiler vectorizes each chunk into 16-bit multiplies and pairwise
        // multiply-adds (`vpmullw` and `vpmaddwd` on x86-64-v3), 16 values an
        // instruction.
        let qa = QA as i16;
        let side = |acc: &[i16; H], weights: &[i16; H]| -> i64 {
            acc.chunks(CHUNK)
                .zip(weights.chunks(CHUNK))
                .map(|(a, w)| {
                    let mut sum = 0i32;
                    for (&x, &w) in a.iter().zip(w) {
                        let c = x.clamp(0, qa);
                        sum += i32::from(c.wrapping_mul(w)) * i32::from(c);
                    }
                    i64::from(sum)
                })
                .sum()
        };
        let sum = side(&us.0, &self.output_us) + side(&them.0, &self.output_them);
        let out = (sum / i64::from(QA) + i64::from(self.output_bias)) * i64::from(SCALE)
            / i64::from(QA * QB);
        out as i32
    }

    /// The accumulator for `perspective`, computed from scratch.
    pub fn refresh(&self, pos: &Position, perspective: Color) -> Acc<H> {
        let mut acc = Acc(self.feature_bias);
        for sq in Square::ALL {
            if let Some(p) = pos.piece_at(sq) {
                acc.add(self, feature(perspective, p.color, p.kind, sq));
            }
        }
        acc
    }
}

/// A network of any width the engine reads.
// The wider variants are larger inline (a few KB of biases and output weights
// more), but a `Network` always lives in an `Arc` on the heap, and boxing them
// would add a pointer hop to every evaluation.
#[allow(clippy::large_enum_variant)]
pub enum Network {
    /// 128 hidden neurons per perspective.
    H128(Net<128>),
    /// 256 hidden neurons per perspective.
    H256(Net<256>),
    /// 512 hidden neurons per perspective.
    H512(Net<512>),
}

impl Network {
    /// Read a network from its file bytes (see the module documentation);
    /// the width is the one whose size the file has (a trailing pad of up to
    /// 63 bytes is allowed).
    pub fn from_bytes(bytes: &[u8]) -> Result<Network, String> {
        let width = WIDTHS
            .iter()
            .copied()
            .find(|&h| bytes.len() >= file_bytes(h) && bytes.len() - file_bytes(h) < 64)
            .ok_or_else(|| {
                let sizes: Vec<String> = WIDTHS
                    .iter()
                    .map(|&h| format!("{INPUTS} → {h} × 2 → 1 ({} bytes)", file_bytes(h)))
                    .collect();
                format!(
                    "a network is {}, plus up to 63 bytes of padding; this file is {} bytes",
                    sizes.join(" or "),
                    bytes.len()
                )
            })?;
        let mut words = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]));
        let mut next = || words.next().expect("length checked above");
        Ok(match width {
            128 => Network::H128(Net::from_words(&mut next)?),
            256 => Network::H256(Net::from_words(&mut next)?),
            _ => Network::H512(Net::from_words(&mut next)?),
        })
    }

    /// Hidden neurons per perspective.
    pub fn hidden(&self) -> usize {
        match self {
            Network::H128(_) => 128,
            Network::H256(_) => 256,
            Network::H512(_) => 512,
        }
    }

    /// The file bytes of this network (for tests and tools).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(file_bytes(self.hidden()));
        match self {
            Network::H128(n) => n.put_words(&mut out),
            Network::H256(n) => n.put_words(&mut out),
            Network::H512(n) => n.put_words(&mut out),
        }
        out
    }

    /// A 128-neuron network from explicit weights, the output weights as one
    /// array: the side to move's half, then the other's (for tests).
    pub fn from_parts(
        feature_weights: Box<[[i16; HIDDEN]; INPUTS]>,
        feature_bias: [i16; HIDDEN],
        output_weights: [i16; 2 * HIDDEN],
        output_bias: i16,
    ) -> Network {
        let (us, them) = output_weights.split_at(HIDDEN);
        Network::H128(Net::from_parts(
            feature_weights,
            feature_bias,
            us.try_into().unwrap(),
            them.try_into().unwrap(),
            output_bias,
        ))
    }

    /// The evaluation from accumulators given as plain values (for tests and
    /// tools; the search uses [`NnueState`]).
    pub fn evaluate_values(&self, us: &[i16], them: &[i16]) -> i32 {
        fn at<const H: usize>(n: &Net<H>, us: &[i16], them: &[i16]) -> i32 {
            n.evaluate(&Acc(us.try_into().unwrap()), &Acc(them.try_into().unwrap()))
        }
        match self {
            Network::H128(n) => at(n, us, them),
            Network::H256(n) => at(n, us, them),
            Network::H512(n) => at(n, us, them),
        }
    }

    /// The accumulator for `perspective`, computed from scratch, as plain
    /// values (for tests and tools).
    pub fn refresh_values(&self, pos: &Position, perspective: Color) -> Vec<i16> {
        match self {
            Network::H128(n) => n.refresh(pos, perspective).0.to_vec(),
            Network::H256(n) => n.refresh(pos, perspective).0.to_vec(),
            Network::H512(n) => n.refresh(pos, perspective).0.to_vec(),
        }
    }
}

/// The feature index of a piece from a perspective (see the module
/// documentation).
#[inline]
pub fn feature(perspective: Color, color: Color, kind: PieceKind, sq: Square) -> usize {
    let (theirs, sq) = if perspective == Color::White {
        (color != Color::White, sq.index())
    } else {
        (color != Color::Black, sq.index() ^ 56)
    };
    usize::from(theirs) * 384 + kind.index() * 64 + sq
}

/// One perspective's hidden layer before activation, at width `H`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Acc<const H: usize>(pub [i16; H]);

/// A 128-neuron accumulator.
pub type Accumulator = Acc<HIDDEN>;

impl<const H: usize> Acc<H> {
    /// Add a feature's weight column.
    #[inline]
    pub fn add(&mut self, net: &Net<H>, feature: usize) {
        let col = &net.feature_weights[feature];
        for (a, w) in self.0.iter_mut().zip(col) {
            *a = a.wrapping_add(*w);
        }
    }

    /// Subtract a feature's weight column.
    #[inline]
    pub fn sub(&mut self, net: &Net<H>, feature: usize) {
        let col = &net.feature_weights[feature];
        for (a, w) in self.0.iter_mut().zip(col) {
            *a = a.wrapping_sub(*w);
        }
    }

    // The fused updates below write the next ply's accumulator from the
    // current one in a single pass, instead of a copy and then one pass per
    // moved piece. Wrapping arithmetic is associative, so the result is the
    // same as adding and subtracting the columns one at a time.

    /// `old` plus `add`'s column minus `sub`'s: a quiet move.
    #[inline]
    pub fn add_sub(&mut self, old: &Acc<H>, net: &Net<H>, add: usize, sub: usize) {
        let (a, s) = (&net.feature_weights[add], &net.feature_weights[sub]);
        for i in 0..H {
            self.0[i] = old.0[i].wrapping_add(a[i]).wrapping_sub(s[i]);
        }
    }

    /// `old` plus `add`'s column minus `sub1`'s and `sub2`'s: a capture.
    #[inline]
    pub fn add_sub_sub(
        &mut self,
        old: &Acc<H>,
        net: &Net<H>,
        add: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let a = &net.feature_weights[add];
        let (s1, s2) = (&net.feature_weights[sub1], &net.feature_weights[sub2]);
        for i in 0..H {
            self.0[i] = old.0[i]
                .wrapping_add(a[i])
                .wrapping_sub(s1[i])
                .wrapping_sub(s2[i]);
        }
    }

    /// `old` plus two columns minus two: castling.
    #[inline]
    pub fn add_add_sub_sub(
        &mut self,
        old: &Acc<H>,
        net: &Net<H>,
        [add1, add2]: [usize; 2],
        [sub1, sub2]: [usize; 2],
    ) {
        let (a1, a2) = (&net.feature_weights[add1], &net.feature_weights[add2]);
        let (s1, s2) = (&net.feature_weights[sub1], &net.feature_weights[sub2]);
        for i in 0..H {
            self.0[i] = old.0[i]
                .wrapping_add(a1[i])
                .wrapping_add(a2[i])
                .wrapping_sub(s1[i])
                .wrapping_sub(s2[i]);
        }
    }
}

/// What a move changes on the board, for the accumulators.
#[derive(Clone, Copy, Debug)]
pub(crate) struct MoveDelta {
    /// The side that moved.
    pub us: Color,
    /// The piece that left `from`.
    pub moved: PieceKind,
    /// The piece that arrived on `to` (the promoted piece on a promotion).
    pub placed: PieceKind,
    pub from: Square,
    pub to: Square,
    /// The piece taken and its square (not `to` en passant).
    pub captured: Option<(Color, PieceKind, Square)>,
    /// The rook's from and to squares when castling.
    pub castle: Option<(Square, Square)>,
}

/// A network and the accumulators of the current line, one entry per ply:
/// pushed by make, popped by unmake. Nothing here allocates after `new`.
#[derive(Clone)]
pub(crate) struct NnueState {
    net: Arc<Network>,
    stack: Stack,
    top: usize,
}

/// `[white's perspective, black's]` per ply, at the network's width.
#[derive(Clone)]
enum Stack {
    H128(Box<[[Acc<128>; 2]]>),
    H256(Box<[[Acc<256>; 2]]>),
    H512(Box<[[Acc<512>; 2]]>),
}

fn new_stack<const H: usize>(net: &Net<H>, pos: &Position, len: usize) -> Box<[[Acc<H>; 2]]> {
    let mut stack = vec![[Acc([0; H]); 2]; len].into_boxed_slice();
    stack[0] = [
        net.refresh(pos, Color::White),
        net.refresh(pos, Color::Black),
    ];
    stack
}

#[inline]
fn push_move<const H: usize>(
    net: &Net<H>,
    stack: &mut [[Acc<H>; 2]],
    top: &mut usize,
    d: &MoveDelta,
) {
    // Outside a search (a long game played move by move) the stack can fill;
    // it then restarts from the bottom, which is safe because a game is never
    // unmade there.
    if *top + 1 == stack.len() {
        stack[0] = stack[*top];
        *top = 0;
    }
    // The next ply's accumulators, from the current ones in one pass per
    // perspective.
    let (below, above) = stack.split_at_mut(*top + 1);
    let (old, new) = (&below[*top], &mut above[0]);
    for (i, view) in [Color::White, Color::Black].into_iter().enumerate() {
        let on = feature(view, d.us, d.placed, d.to);
        let off = feature(view, d.us, d.moved, d.from);
        match (d.captured, d.castle) {
            (Some((color, kind, sq)), _) => {
                new[i].add_sub_sub(&old[i], net, on, off, feature(view, color, kind, sq))
            }
            (None, Some((rook_from, rook_to))) => new[i].add_add_sub_sub(
                &old[i],
                net,
                [on, feature(view, d.us, PieceKind::Rook, rook_to)],
                [off, feature(view, d.us, PieceKind::Rook, rook_from)],
            ),
            (None, None) => new[i].add_sub(&old[i], net, on, off),
        }
    }
    *top += 1;
}

impl NnueState {
    /// The accumulators of `pos`, computed from scratch, with room for `len`
    /// plies.
    pub(crate) fn new(net: Arc<Network>, pos: &Position, len: usize) -> NnueState {
        let stack = match &*net {
            Network::H128(n) => Stack::H128(new_stack(n, pos, len)),
            Network::H256(n) => Stack::H256(new_stack(n, pos, len)),
            Network::H512(n) => Stack::H512(new_stack(n, pos, len)),
        };
        NnueState { net, stack, top: 0 }
    }

    /// The evaluation for the side to move.
    #[inline]
    pub(crate) fn evaluate(&self, stm: Color) -> i32 {
        let w = usize::from(stm != Color::White);
        match (&*self.net, &self.stack) {
            (Network::H128(n), Stack::H128(s)) => n.evaluate(&s[self.top][w], &s[self.top][1 - w]),
            (Network::H256(n), Stack::H256(s)) => n.evaluate(&s[self.top][w], &s[self.top][1 - w]),
            (Network::H512(n), Stack::H512(s)) => n.evaluate(&s[self.top][w], &s[self.top][1 - w]),
            _ => unreachable!("the stack has the network's width"),
        }
    }

    /// Push the accumulators after a move.
    #[inline]
    pub(crate) fn push(&mut self, d: &MoveDelta) {
        match (&*self.net, &mut self.stack) {
            (Network::H128(n), Stack::H128(s)) => push_move(n, s, &mut self.top, d),
            (Network::H256(n), Stack::H256(s)) => push_move(n, s, &mut self.top, d),
            (Network::H512(n), Stack::H512(s)) => push_move(n, s, &mut self.top, d),
            _ => unreachable!("the stack has the network's width"),
        }
    }

    /// Back to the previous ply's accumulators.
    #[inline]
    pub(crate) fn pop(&mut self) {
        debug_assert!(self.top > 0, "unmake past the accumulator stack's bottom");
        self.top -= 1;
    }

    /// Move the current accumulators to the bottom of the stack (a new
    /// search root).
    #[inline]
    pub(crate) fn set_root(&mut self) {
        match &mut self.stack {
            Stack::H128(s) => s[0] = s[self.top],
            Stack::H256(s) => s[0] = s[self.top],
            Stack::H512(s) => s[0] = s[self.top],
        }
        self.top = 0;
    }

    /// The current accumulators, `[white's perspective, black's]`, as plain
    /// values (for tests).
    pub(crate) fn values(&self) -> [Vec<i16>; 2] {
        match &self.stack {
            Stack::H128(s) => [s[self.top][0].0.to_vec(), s[self.top][1].0.to_vec()],
            Stack::H256(s) => [s[self.top][0].0.to_vec(), s[self.top][1].0.to_vec()],
            Stack::H512(s) => [s[self.top][0].0.to_vec(), s[self.top][1].0.to_vec()],
        }
    }
}
