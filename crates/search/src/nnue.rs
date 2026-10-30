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

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind, Square};

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
        let screlu = |x: i16| {
            let c = i32::from(x).clamp(0, QA);
            c * c
        };
        // i64: QA² · 127 (the trainer's weight clip at QB = 64) over 2 · HIDDEN
        // terms comes within a few percent of i32's range at 128 neurons.
        let mut sum = 0i64;
        for i in 0..HIDDEN {
            sum += i64::from(screlu(us.0[i])) * i64::from(self.output_weights[i]);
            sum += i64::from(screlu(them.0[i])) * i64::from(self.output_weights[HIDDEN + i]);
        }
        let out = (sum / i64::from(QA) + i64::from(self.output_bias)) * i64::from(SCALE)
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
}
