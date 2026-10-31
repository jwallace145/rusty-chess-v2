//! NNUE evaluation: a (768 → [`HIDDEN`]) × 2 → 1 network with squared
//! clipped ReLU, quantized to `i16`, evaluated from an accumulator that make
//! and unmake update incrementally.
//!
//! **Inputs.** 768 features per perspective: one for each colour, piece kind
//! and square. From a perspective, "our" pieces are features 0–383 and the
//! opponent's 384–767, each block ordered pawn, knight, bishop, rook, queen,
//! king, 64 squares each; black's perspective flips the board vertically
//! (square ^ 56). This is `bullet`'s `Chess768` layout.
//!
//! **Hidden layer.** One accumulator of [`HIDDEN`] `i16` values per
//! perspective: the feature bias plus the weight column of every active
//! feature. A move only adds and subtracts a few columns.
//!
//! **Output.** For the side to move, `screlu(ours) · w[..H] +
//! screlu(theirs) · w[H..]`, scaled back by `QA`, plus the output bias, times
//! [`SCALE`] over `QA · QB`, in centipawns. `screlu(x) = clamp(x, 0, QA)²`.
//!
//! **File format** (all `i16`, little-endian, in this order; a trailing pad
//! to a multiple of 64 bytes is allowed): feature weights `[768][HIDDEN]`,
//! feature bias `[HIDDEN]`, output weights `[2 * HIDDEN]` (ours, then
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
/// (`datagen-audit`) and so they can be selected by id for comparisons. At
/// 128 hidden neurons a net is about 200 KB.
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
        id: "net3-l050-r41160495666",
        generation: 3,
        bytes: include_bytes!("../nets/net3-l050-r41160495666.nnue"),
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

/// Hidden neurons per perspective. Must match the net being loaded; raised
/// with the nets.
pub const HIDDEN: usize = 128;

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
/// exact in 16- and 32-bit arithmetic (see [`Network::evaluate`]).
pub const MAX_OUTPUT_WEIGHT: i16 = 127;

/// The size in bytes of a network file without padding.
pub const FILE_BYTES: usize = 2 * (INPUTS * HIDDEN + HIDDEN + 2 * HIDDEN + 1);

/// A network's quantized weights.
pub struct Network {
    /// `[INPUTS][HIDDEN]`, feature-major: one column per feature.
    feature_weights: Box<[[i16; HIDDEN]; INPUTS]>,
    feature_bias: [i16; HIDDEN],
    output_weights: [i16; 2 * HIDDEN],
    output_bias: i16,
}

impl Network {
    /// Read a network from its file bytes (see the module documentation).
    pub fn from_bytes(bytes: &[u8]) -> Result<Network, String> {
        if bytes.len() < FILE_BYTES || bytes.len() - FILE_BYTES >= 64 {
            return Err(format!(
                "a {INPUTS} → {HIDDEN} × 2 → 1 network is {FILE_BYTES} bytes (plus up to 63 of padding); this file is {}",
                bytes.len()
            ));
        }
        let mut words = bytes
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]));
        let mut next = || words.next().expect("length checked above");
        let mut feature_weights: Box<[[i16; HIDDEN]; INPUTS]> = vec![[0i16; HIDDEN]; INPUTS]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!("the vector has INPUTS rows"));
        for row in feature_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = next();
            }
        }
        let mut feature_bias = [0i16; HIDDEN];
        feature_bias.iter_mut().for_each(|b| *b = next());
        let mut output_weights = [0i16; 2 * HIDDEN];
        output_weights.iter_mut().for_each(|w| *w = next());
        let output_bias = next();
        if let Some(w) = output_weights.iter().find(|w| w.abs() > MAX_OUTPUT_WEIGHT) {
            return Err(format!(
                "an output weight is {w}, beyond ±{MAX_OUTPUT_WEIGHT}; this is not a net the trainer saved"
            ));
        }
        Ok(Network {
            feature_weights,
            feature_bias,
            output_weights,
            output_bias,
        })
    }

    /// The file bytes of this network (for tests and tools).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FILE_BYTES);
        let mut put = |w: i16| out.extend_from_slice(&w.to_le_bytes());
        self.feature_weights.iter().flatten().for_each(|&w| put(w));
        self.feature_bias.iter().for_each(|&w| put(w));
        self.output_weights.iter().for_each(|&w| put(w));
        put(self.output_bias);
        out
    }

    /// A network from explicit weights (for tests and tools).
    pub fn from_parts(
        feature_weights: Box<[[i16; HIDDEN]; INPUTS]>,
        feature_bias: [i16; HIDDEN],
        output_weights: [i16; 2 * HIDDEN],
        output_bias: i16,
    ) -> Network {
        assert!(
            output_weights.iter().all(|w| w.abs() <= MAX_OUTPUT_WEIGHT),
            "output weights must be within ±{MAX_OUTPUT_WEIGHT}"
        );
        Network {
            feature_weights,
            feature_bias,
            output_weights,
            output_bias,
        }
    }

    /// The evaluation in centipawns for the side to move, from the
    /// accumulators of the side to move (`us`) and its opponent (`them`).
    pub fn evaluate(&self, us: &Accumulator, them: &Accumulator) -> i32 {
        // screlu(x) · w computed as (c · w) · c with c = clamp(x, 0, QA). With
        // every output weight within ±MAX_OUTPUT_WEIGHT (enforced at load),
        // c · w fits an i16 (255 · 127 = 32,385) and the whole sum an i32
        // (255² · 127 · 256 = 2,114,092,800), so this is exact: the same
        // result as summing in i64. In this shape the compiler vectorizes it
        // into 16-bit multiplies and pairwise multiply-adds (`vpmullw` and
        // `vpmaddwd` on x86-64-v3), 16 values an instruction.
        let qa = QA as i16;
        let (w_us, w_them) = self.output_weights.split_at(HIDDEN);
        let mut sum = 0i32;
        for i in 0..HIDDEN {
            let u = us.0[i].clamp(0, qa);
            let t = them.0[i].clamp(0, qa);
            sum += i32::from(u.wrapping_mul(w_us[i])) * i32::from(u);
            sum += i32::from(t.wrapping_mul(w_them[i])) * i32::from(t);
        }
        let out = (i64::from(sum) / i64::from(QA) + i64::from(self.output_bias)) * i64::from(SCALE)
            / i64::from(QA * QB);
        out as i32
    }

    /// The accumulator for `perspective`, computed from scratch.
    pub fn refresh(&self, pos: &Position, perspective: Color) -> Accumulator {
        let mut acc = Accumulator(self.feature_bias);
        for sq in Square::ALL {
            if let Some(p) = pos.piece_at(sq) {
                acc.add(self, feature(perspective, p.color, p.kind, sq));
            }
        }
        acc
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

/// One perspective's hidden layer before activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accumulator(pub [i16; HIDDEN]);

impl Accumulator {
    /// Add a feature's weight column.
    #[inline]
    pub fn add(&mut self, net: &Network, feature: usize) {
        let col = &net.feature_weights[feature];
        for (a, w) in self.0.iter_mut().zip(col) {
            *a = a.wrapping_add(*w);
        }
    }

    /// Subtract a feature's weight column.
    #[inline]
    pub fn sub(&mut self, net: &Network, feature: usize) {
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
    pub fn add_sub(&mut self, old: &Accumulator, net: &Network, add: usize, sub: usize) {
        let (a, s) = (&net.feature_weights[add], &net.feature_weights[sub]);
        for i in 0..HIDDEN {
            self.0[i] = old.0[i].wrapping_add(a[i]).wrapping_sub(s[i]);
        }
    }

    /// `old` plus `add`'s column minus `sub1`'s and `sub2`'s: a capture.
    #[inline]
    pub fn add_sub_sub(
        &mut self,
        old: &Accumulator,
        net: &Network,
        add: usize,
        sub1: usize,
        sub2: usize,
    ) {
        let a = &net.feature_weights[add];
        let (s1, s2) = (&net.feature_weights[sub1], &net.feature_weights[sub2]);
        for i in 0..HIDDEN {
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
        old: &Accumulator,
        net: &Network,
        [add1, add2]: [usize; 2],
        [sub1, sub2]: [usize; 2],
    ) {
        let (a1, a2) = (&net.feature_weights[add1], &net.feature_weights[add2]);
        let (s1, s2) = (&net.feature_weights[sub1], &net.feature_weights[sub2]);
        for i in 0..HIDDEN {
            self.0[i] = old.0[i]
                .wrapping_add(a1[i])
                .wrapping_add(a2[i])
                .wrapping_sub(s1[i])
                .wrapping_sub(s2[i]);
        }
    }
}
