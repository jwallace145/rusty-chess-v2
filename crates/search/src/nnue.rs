//! NNUE evaluation: a (768 × K → H) × 2 → B network with squared clipped
//! ReLU, quantized to `i16`, evaluated from an accumulator that make and
//! unmake update incrementally. H is 128, 256, 512 or 768 ([`WIDTHS`]), B, the
//! output buckets, 1 or 8 ([`OUTPUT_BUCKETS`]), and K, the king buckets, 1 or
//! 4 ([`KING_BUCKETS`], `docs/king-buckets.md`), all read from the file's size; each
//! width's code is compiled separately ([`Net`], [`Acc`]) and chosen once per
//! call ([`Network`], `NnueState`).
//!
//! **Inputs.** 768 features per perspective: one for each colour, piece kind
//! and square. From a perspective, "our" pieces are features 0–383 and the
//! opponent's 384–767, each block ordered pawn, knight, bishop, rook, queen,
//! king, 64 squares each; black's perspective flips the board vertically
//! (square ^ 56). This is `bullet`'s `Chess768` layout.
//!
//! **King buckets.** With K = 4, each perspective's features are indexed in
//! its *view*: the bucket of its own king's square ([`KING_BUCKET_LAYOUT`])
//! and, with the king on files e–h, the board mirrored left to right
//! (`index ^ 7`). This is `bullet`'s `ChessBucketsMirrored`. K = 1 is a single
//! view, today's indexing.
//!
//! **Hidden layer.** One accumulator of H `i16` values per
//! perspective: the feature bias plus the weight column of every active
//! feature. A move only adds and subtracts a few columns; a king move that
//! changes its side's view rebuilds that side's accumulator.
//!
//! **Output.** For the side to move, `screlu(ours) · w[b][..H] +
//! screlu(theirs) · w[b][H..]`, scaled back by `QA`, plus the output bias
//! `bias[b]`, times [`SCALE`] over `QA · QB`, in centipawns.
//! `screlu(x) = clamp(x, 0, QA)²`. The bucket `b` comes from the number of
//! pieces on the board ([`output_bucket`]); a single-bucket net always uses
//! bucket 0.
//!
//! **File format** (all `i16`, little-endian, in this order; a trailing pad
//! to a multiple of 64 bytes is allowed): feature weights `[768 × K][H]`
//! (bucket 0's 768 rows first),
//! feature bias `[H]`, output weights `[B][2 * H]` (per bucket: ours, then
//! theirs), output biases `[B]`. At B = 1 this is what `bullet`'s simple example saves
//! for this architecture at `QA = 255`, `QB = 64`, `SCALE = 400`; the first
//! trained net is checked against the trainer's own evaluation of a few
//! positions before it is used (see `docs/nnue.md`).

use std::sync::{Arc, OnceLock};

use rusty_chess_core::bitboard::Bitboard;
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

/// The nets the binary carries: only the default, the last entry. A net is
/// about 200 KB at 128 hidden neurons, 400 KB at 256 and 790 KB at 512 (800
/// KB with 8 output buckets, 3.2 MB with 4 king buckets as well), so
/// carrying every generation's net grew the binary with each one. A new
/// default replaces the entry; the one it replaces moves to [`RETIRED_NETS`].
pub const NETS: [EmbeddedNet; 1] = [EmbeddedNet {
    id: "net8-l050-r42564788061",
    generation: 7,
    bytes: include_bytes!("../nets/net8-l050-r42564788061.nnue"),
}];

/// Where every trained net is kept, forever: `<NET_STORE>/<id>/net.nnue`,
/// beside its `training.json`.
pub const NET_STORE: &str = "s3://jwall-engineering-rusty-chess-nnue/data/nets";

/// A net that played an earlier generation and is no longer embedded. Its
/// file is in [`NET_STORE`], byte for byte the file with this SHA-256, and
/// the engine of the commit that generated its data embeds it (the audit
/// re-searches a generation with a build at that commit).
pub struct RetiredNet {
    /// Its id (`docs/nets.md`).
    pub id: &'static str,
    /// The self-play generation it played.
    pub generation: u32,
    /// The SHA-256 of its `net.nnue`.
    pub sha256: &'static str,
}

/// Every generation's net before the default's, oldest first.
pub const RETIRED_NETS: [RetiredNet; 6] = [
    RetiredNet {
        id: "net1-l050-r41022503872",
        generation: 1,
        sha256: "dffdc62a25fdd7f0a638db07f24bb726f08921979e09298ddff2b5fe1f3df57b",
    },
    RetiredNet {
        id: "net2x80-l050-r41131366975",
        generation: 2,
        sha256: "731b30d205c505ae13e9f7a30772c1b02325a2ce00458bb42f4ab397eb36677e",
    },
    RetiredNet {
        id: "net4w512-l050-r41170687764",
        generation: 3,
        sha256: "5d24472c5dcc3671f42ee86b88af6973d35234aa490d4c7eec04ca6aa71b5768",
    },
    RetiredNet {
        id: "net5-l050-r41229287546",
        generation: 4,
        sha256: "acd8e0496be9737132bbe8260d4e89b3104b54da1f1e60b4a0f4c7f8abbd906b",
    },
    RetiredNet {
        id: "net6k4-l050-r41282883885",
        generation: 5,
        sha256: "82a7f9869e6c2104c0858ab4f6d5a8e57ca943449254aad3ee384dfedd20df33",
    },
    RetiredNet {
        id: "net7rl012-l050-r42054816508",
        generation: 6,
        sha256: "74af5b4645aa340d10f3c5e79d61fe4b0975d9dea0cf44b7e7024b9eea3f70ce",
    },
];

/// The retired net with this id, if any.
pub fn retired_net(id: &str) -> Option<&'static RetiredNet> {
    RETIRED_NETS.iter().find(|n| n.id == id)
}

/// Why `id` can't be loaded by id: a retired net, and where its file is.
pub fn retired_net_message(net: &RetiredNet) -> String {
    format!(
        "{} played generation {} and is no longer embedded; download {NET_STORE}/{}/net.nnue (sha256 {}) and pass the file",
        net.id, net.generation, net.id, net.sha256
    )
}

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
pub const WIDTHS: [usize; 4] = [128, 256, 512, 768];

/// The output bucket counts the engine reads, told apart by file size: one
/// output layer, or one per band of piece counts ([`output_bucket`]).
pub const OUTPUT_BUCKETS: [usize; 2] = [1, 8];

/// The most output buckets a net may have.
pub const MAX_OUTPUT_BUCKETS: usize = 8;

/// The king bucket counts the engine reads, told apart by file size: one
/// view (no king buckets), or [`KING_BUCKET_LAYOUT`]'s four.
pub const KING_BUCKETS: [usize; 2] = [1, 4];

/// The king bucket of each square a mirrored king can stand on, from its own
/// side of the board: files a–d of ranks 1 to 8, four to a rank. Files e–h
/// mirror these. Part of the file format: a 4-king-bucket net always means
/// this table (`docs/king-buckets.md`).
#[rustfmt::skip]
pub const KING_BUCKET_LAYOUT: [usize; 32] = [
    0, 0, 1, 1,
    2, 2, 2, 2,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
    3, 3, 3, 3,
];

/// A perspective's view in a net of `king_buckets`: the offset of its king
/// bucket's features and the XOR that mirrors a square's file, from its own
/// king's square `ksq` seen from its own side (`square ^ 56` for black).
#[inline]
pub const fn king_view(king_buckets: usize, ksq: usize) -> (usize, usize) {
    if king_buckets == 1 {
        return (0, 0);
    }
    let file = ksq % 8;
    let (flip, half) = if file > 3 { (7, 7 - file) } else { (0, file) };
    (INPUTS * KING_BUCKET_LAYOUT[(ksq / 8) * 4 + half], flip)
}

/// `color`'s king square seen from its own side of the board.
#[inline]
fn own_king_square(pos: &Position, color: Color) -> usize {
    let sq = pos.king_square(color).index();
    if color == Color::White { sq } else { sq ^ 56 }
}

/// The output bucket of a position with `pieces` pieces on the board (kings
/// and pawns included) in a net of `buckets` buckets: the piece counts 2..=32
/// split into `buckets` bands of `32 / buckets`, fewest pieces first. At 8
/// buckets, 2–5 pieces are bucket 0 and 30–32 bucket 7. This is `bullet`'s
/// `MaterialCount<buckets>`. A single-bucket net is always bucket 0.
#[inline]
pub const fn output_bucket(pieces: u32, buckets: usize) -> usize {
    let band = 32usize.div_ceil(buckets);
    let b = (pieces as usize).saturating_sub(2) / band;
    if b < buckets { b } else { buckets - 1 }
}

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

/// The size in bytes of a single-bucket network file of width `hidden`,
/// without padding.
pub const fn file_bytes(hidden: usize) -> usize {
    bucketed_file_bytes(hidden, 1)
}

/// The size in bytes of a network file of width `hidden` with `buckets`
/// output buckets (one view), without padding.
pub const fn bucketed_file_bytes(hidden: usize, buckets: usize) -> usize {
    net_file_bytes(hidden, buckets, 1)
}

/// The size in bytes of a network file of width `hidden`, `buckets` output
/// buckets and `king_buckets` king buckets, without padding.
pub const fn net_file_bytes(hidden: usize, buckets: usize, king_buckets: usize) -> usize {
    2 * (INPUTS * king_buckets * hidden + hidden + buckets * (2 * hidden + 1))
}

/// The size in bytes of a 128-neuron network file without padding.
pub const FILE_BYTES: usize = file_bytes(HIDDEN);

/// The longest run of output terms summed in `i32` (see [`Net::evaluate`]).
const CHUNK: usize = 256;

/// A network's quantized weights at width `H`.
pub struct Net<const H: usize> {
    /// `[INPUTS × king_buckets][H]`, feature-major: one column per feature,
    /// king bucket 0's first.
    feature_weights: Box<[[i16; H]]>,
    /// King buckets, one of [`KING_BUCKETS`].
    king_buckets: usize,
    feature_bias: [i16; H],
    /// The side to move's half of each bucket's output weights; buckets past
    /// `buckets` are zero and never read.
    output_us: [[i16; H]; MAX_OUTPUT_BUCKETS],
    /// The other side's half.
    output_them: [[i16; H]; MAX_OUTPUT_BUCKETS],
    output_bias: [i16; MAX_OUTPUT_BUCKETS],
    /// Output buckets in use, one of [`OUTPUT_BUCKETS`].
    buckets: usize,
}

impl<const H: usize> Net<H> {
    /// Widths are multiples of 16 (SIMD); `evaluate` stays exact at any of
    /// them by summing in chunks of `CHUNK`.
    const VALID: () = assert!(H % 16 == 0 && H > 0);

    fn from_words(
        next: &mut impl FnMut() -> i16,
        buckets: usize,
        king_buckets: usize,
    ) -> Result<Net<H>, String> {
        let () = Self::VALID;
        let mut feature_weights = vec![[0i16; H]; INPUTS * king_buckets].into_boxed_slice();
        for row in feature_weights.iter_mut() {
            for w in row.iter_mut() {
                *w = next();
            }
        }
        let mut feature_bias = [0i16; H];
        feature_bias.iter_mut().for_each(|b| *b = next());
        let mut output_us = [[0i16; H]; MAX_OUTPUT_BUCKETS];
        let mut output_them = [[0i16; H]; MAX_OUTPUT_BUCKETS];
        for b in 0..buckets {
            output_us[b].iter_mut().for_each(|w| *w = next());
            output_them[b].iter_mut().for_each(|w| *w = next());
        }
        let mut output_bias = [0i16; MAX_OUTPUT_BUCKETS];
        output_bias[..buckets].iter_mut().for_each(|w| *w = next());
        if let Some(w) = output_us
            .iter()
            .chain(&output_them)
            .flatten()
            .find(|w| w.abs() > MAX_OUTPUT_WEIGHT)
        {
            return Err(format!(
                "an output weight is {w}, beyond ±{MAX_OUTPUT_WEIGHT}; this is not a net the trainer saved"
            ));
        }
        Ok(Net {
            feature_weights,
            king_buckets,
            feature_bias,
            output_us,
            output_them,
            output_bias,
            buckets,
        })
    }

    fn put_words(&self, out: &mut Vec<u8>) {
        let mut put = |w: i16| out.extend_from_slice(&w.to_le_bytes());
        self.feature_weights.iter().flatten().for_each(|&w| put(w));
        self.feature_bias.iter().for_each(|&w| put(w));
        for b in 0..self.buckets {
            self.output_us[b].iter().for_each(|&w| put(w));
            self.output_them[b].iter().for_each(|&w| put(w));
        }
        self.output_bias[..self.buckets]
            .iter()
            .for_each(|&w| put(w));
    }

    /// A single-bucket network from explicit weights (for tests and tools).
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
        let mut net = Net {
            feature_weights,
            king_buckets: 1,
            feature_bias,
            output_us: [[0; H]; MAX_OUTPUT_BUCKETS],
            output_them: [[0; H]; MAX_OUTPUT_BUCKETS],
            output_bias: [0; MAX_OUTPUT_BUCKETS],
            buckets: 1,
        };
        net.output_us[0] = output_us;
        net.output_them[0] = output_them;
        net.output_bias[0] = output_bias;
        net
    }

    /// The evaluation in centipawns for the side to move, from the
    /// accumulators of the side to move (`us`) and its opponent (`them`), in
    /// a position with `pieces` pieces on the board.
    #[inline]
    pub fn evaluate(&self, us: &Acc<H>, them: &Acc<H>, pieces: u32) -> i32 {
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
        let b = output_bucket(pieces, self.buckets);
        let sum = side(&us.0, &self.output_us[b]) + side(&them.0, &self.output_them[b]);
        let out = (sum / i64::from(QA) + i64::from(self.output_bias[b])) * i64::from(SCALE)
            / i64::from(QA * QB);
        out as i32
    }

    /// `perspective`'s view in `pos` (see [`king_view`]).
    #[inline]
    fn view(&self, pos: &Position, perspective: Color) -> (usize, usize) {
        if self.king_buckets == 1 {
            return (0, 0);
        }
        king_view(self.king_buckets, own_king_square(pos, perspective))
    }

    /// The accumulator for `perspective`, computed from scratch.
    pub fn refresh(&self, pos: &Position, perspective: Color) -> Acc<H> {
        let view = self.view(pos, perspective);
        let mut acc = Acc(self.feature_bias);
        for sq in Square::ALL {
            if let Some(p) = pos.piece_at(sq) {
                acc.add(self, feature_in(view, perspective, p.color, p.kind, sq));
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
    /// 768 hidden neurons per perspective.
    H768(Net<768>),
}

impl Network {
    /// Read a network from its file bytes (see the module documentation);
    /// the width, the output buckets and the king buckets are the ones whose
    /// size the file has (a trailing pad of up to 63 bytes is allowed).
    /// Every combination's size is thousands of bytes from every other's, so
    /// at most one fits.
    pub fn from_bytes(bytes: &[u8]) -> Result<Network, String> {
        let shapes = || {
            WIDTHS.iter().flat_map(|&h| {
                OUTPUT_BUCKETS
                    .iter()
                    .flat_map(move |&b| KING_BUCKETS.iter().map(move |&k| (h, b, k)))
            })
        };
        let (width, buckets, king_buckets) = shapes()
            .find(|&(h, b, k)| {
                let size = net_file_bytes(h, b, k);
                bytes.len() >= size && bytes.len() - size < 64
            })
            .ok_or_else(|| {
                let sizes: Vec<String> = shapes()
                    .map(|(h, b, k)| {
                        format!(
                            "{INPUTS} × {k} → {h} × 2 → {b} ({} bytes)",
                            net_file_bytes(h, b, k)
                        )
                    })
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
            128 => Network::H128(Net::from_words(&mut next, buckets, king_buckets)?),
            256 => Network::H256(Net::from_words(&mut next, buckets, king_buckets)?),
            512 => Network::H512(Net::from_words(&mut next, buckets, king_buckets)?),
            _ => Network::H768(Net::from_words(&mut next, buckets, king_buckets)?),
        })
    }

    /// King buckets: 1, or [`KING_BUCKET_LAYOUT`]'s four.
    pub fn king_buckets(&self) -> usize {
        match self {
            Network::H128(n) => n.king_buckets,
            Network::H256(n) => n.king_buckets,
            Network::H512(n) => n.king_buckets,
            Network::H768(n) => n.king_buckets,
        }
    }

    /// Output buckets: 1, or one per band of piece counts.
    pub fn buckets(&self) -> usize {
        match self {
            Network::H128(n) => n.buckets,
            Network::H256(n) => n.buckets,
            Network::H512(n) => n.buckets,
            Network::H768(n) => n.buckets,
        }
    }

    /// Hidden neurons per perspective.
    pub fn hidden(&self) -> usize {
        match self {
            Network::H128(_) => 128,
            Network::H256(_) => 256,
            Network::H512(_) => 512,
            Network::H768(_) => 768,
        }
    }

    /// The file bytes of this network (for tests and tools).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(net_file_bytes(
            self.hidden(),
            self.buckets(),
            self.king_buckets(),
        ));
        match self {
            Network::H128(n) => n.put_words(&mut out),
            Network::H256(n) => n.put_words(&mut out),
            Network::H512(n) => n.put_words(&mut out),
            Network::H768(n) => n.put_words(&mut out),
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

    /// The evaluation from accumulators given as plain values, in a position
    /// with `pieces` pieces on the board (for tests and tools; the search
    /// uses [`NnueState`]).
    pub fn evaluate_values(&self, us: &[i16], them: &[i16], pieces: u32) -> i32 {
        fn at<const H: usize>(n: &Net<H>, us: &[i16], them: &[i16], pieces: u32) -> i32 {
            n.evaluate(
                &Acc(us.try_into().unwrap()),
                &Acc(them.try_into().unwrap()),
                pieces,
            )
        }
        match self {
            Network::H128(n) => at(n, us, them, pieces),
            Network::H256(n) => at(n, us, them, pieces),
            Network::H512(n) => at(n, us, them, pieces),
            Network::H768(n) => at(n, us, them, pieces),
        }
    }

    /// The accumulator for `perspective`, computed from scratch, as plain
    /// values (for tests and tools).
    pub fn refresh_values(&self, pos: &Position, perspective: Color) -> Vec<i16> {
        match self {
            Network::H128(n) => n.refresh(pos, perspective).0.to_vec(),
            Network::H256(n) => n.refresh(pos, perspective).0.to_vec(),
            Network::H512(n) => n.refresh(pos, perspective).0.to_vec(),
            Network::H768(n) => n.refresh(pos, perspective).0.to_vec(),
        }
    }
}

/// The feature index of a piece from a perspective with one view (see the
/// module documentation).
#[inline]
pub fn feature(perspective: Color, color: Color, kind: PieceKind, sq: Square) -> usize {
    feature_in((0, 0), perspective, color, kind, sq)
}

/// The feature index of a piece from a perspective in `view` (its king
/// bucket's offset and file mirror, from [`king_view`]).
#[inline]
pub fn feature_in(
    (base, flip): (usize, usize),
    perspective: Color,
    color: Color,
    kind: PieceKind,
    sq: Square,
) -> usize {
    let (theirs, sq) = if perspective == Color::White {
        (color != Color::White, sq.index())
    } else {
        (color != Color::Black, sq.index() ^ 56)
    };
    base + ((usize::from(theirs) * 384 + kind.index() * 64 + sq) ^ flip)
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
    stats: RebuildStats,
}

/// The accumulators of the line and the refresh cache, at the network's
/// width.
#[derive(Clone)]
enum Stack {
    H128(Plies<128>),
    H256(Plies<256>),
    H512(Plies<512>),
    H768(Plies<768>),
}

/// The views a king-bucketed net has: each bucket, mirrored or not.
const VIEWS: usize = 8;

/// A view's slot in the refresh cache.
#[inline]
fn view_index((base, flip): (usize, usize)) -> usize {
    (base / INPUTS) * 2 + usize::from(flip != 0)
}

/// `[white's perspective, black's]` per ply, and for each perspective one
/// cached accumulator per view.
#[derive(Clone)]
struct Plies<const H: usize> {
    stack: Box<[[Acc<H>; 2]]>,
    cache: Box<[[CacheEntry<H>; VIEWS]; 2]>,
}

impl<const H: usize> Plies<H> {
    fn new(net: &Net<H>, pos: &Position, len: usize) -> Plies<H> {
        let mut stack = vec![[Acc([0; H]); 2]; len].into_boxed_slice();
        stack[0] = [
            net.refresh(pos, Color::White),
            net.refresh(pos, Color::Black),
        ];
        Plies {
            stack,
            cache: Box::new([[CacheEntry::empty(net); VIEWS]; 2]),
        }
    }
}

/// One perspective's accumulator the last time its king stood in a view, and
/// the pieces it reflects, `[color][kind]` in board squares. When the king
/// comes back to the view, only the pieces that moved meanwhile are applied,
/// instead of every piece on the board.
#[derive(Clone, Copy)]
struct CacheEntry<const H: usize> {
    acc: Acc<H>,
    pieces: [[Bitboard; 6]; 2],
}

impl<const H: usize> CacheEntry<H> {
    /// The bias with no pieces, so a view's first rebuild adds every piece,
    /// as a refresh from scratch does.
    fn empty(net: &Net<H>) -> CacheEntry<H> {
        CacheEntry {
            acc: Acc(net.feature_bias),
            pieces: [[Bitboard::EMPTY; 6]; 2],
        }
    }

    /// Bring the entry up to date with `pos` and write it to `out`: the
    /// columns of pieces here now but not then are added, those there then
    /// but not now subtracted. Wrapping addition is associative and
    /// commutative, so the result equals `net.refresh(pos, perspective)`
    /// exactly, whatever the entry held. Returns the columns applied.
    fn rebuild(
        &mut self,
        net: &Net<H>,
        pos: &Position,
        perspective: Color,
        view: (usize, usize),
        out: &mut Acc<H>,
    ) -> usize {
        let mut columns = 0;
        for color in Color::ALL {
            for kind in PieceKind::ALL {
                let now = pos.pieces(color, kind);
                let then = &mut self.pieces[color.index()][kind.index()];
                if now == *then {
                    continue;
                }
                for sq in now & !*then {
                    self.acc
                        .add(net, feature_in(view, perspective, color, kind, sq));
                }
                for sq in *then & !now {
                    self.acc
                        .sub(net, feature_in(view, perspective, color, kind, sq));
                }
                columns += (now ^ *then).count() as usize;
                *then = now;
            }
        }
        *out = self.acc;
        columns
    }
}

/// What the refresh cache did, counted only with the `stats` feature; a
/// zero-sized no-op without it.
#[derive(Clone, Copy, Debug, Default)]
pub struct RebuildStats {
    /// Rebuilds after a king changed its view.
    #[cfg(feature = "stats")]
    pub rebuilds: u64,
    /// Weight columns the rebuilds applied in total. A rebuild from scratch
    /// applies one per piece on the board.
    #[cfg(feature = "stats")]
    pub columns: u64,
}

impl RebuildStats {
    #[inline]
    fn rebuild(&mut self, columns: usize) {
        #[cfg(feature = "stats")]
        {
            self.rebuilds += 1;
            self.columns += columns as u64;
        }
        #[cfg(not(feature = "stats"))]
        let _ = columns;
    }
}

#[inline]
fn push_move<const H: usize>(
    net: &Net<H>,
    plies: &mut Plies<H>,
    top: &mut usize,
    stats: &mut RebuildStats,
    d: &MoveDelta,
    pos: &Position,
) {
    let Plies { stack, cache } = plies;
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
    if net.king_buckets == 1 {
        // A single view: the constant (0, 0) folds away, so this is the
        // incremental update as it was before king buckets.
        for (i, side) in [Color::White, Color::Black].into_iter().enumerate() {
            update(net, &old[i], &mut new[i], d, |c, k, sq| {
                feature_in((0, 0), side, c, k, sq)
            });
        }
        *top += 1;
        return;
    }
    for (i, side) in [Color::White, Color::Black].into_iter().enumerate() {
        // The side's view after the move (`pos` has made it). Only its own
        // king moving can change it; then the accumulator is rebuilt.
        let view = net.view(pos, side);
        if d.us == side && d.moved == PieceKind::King {
            let from = if side == Color::White {
                d.from.index()
            } else {
                d.from.index() ^ 56
            };
            if king_view(net.king_buckets, from) != view {
                let columns = cache[i][view_index(view)].rebuild(net, pos, side, view, &mut new[i]);
                stats.rebuild(columns);
                continue;
            }
        }
        update(net, &old[i], &mut new[i], d, |c, k, sq| {
            feature_in(view, side, c, k, sq)
        });
    }
    *top += 1;
}

/// One perspective's incremental update for a move, with `f` giving each
/// changed piece's feature index in that perspective's view.
#[inline(always)]
fn update<const H: usize>(
    net: &Net<H>,
    old: &Acc<H>,
    new: &mut Acc<H>,
    d: &MoveDelta,
    f: impl Fn(Color, PieceKind, Square) -> usize,
) {
    let on = f(d.us, d.placed, d.to);
    let off = f(d.us, d.moved, d.from);
    match (d.captured, d.castle) {
        (Some((color, kind, sq)), _) => new.add_sub_sub(old, net, on, off, f(color, kind, sq)),
        (None, Some((rook_from, rook_to))) => new.add_add_sub_sub(
            old,
            net,
            [on, f(d.us, PieceKind::Rook, rook_to)],
            [off, f(d.us, PieceKind::Rook, rook_from)],
        ),
        (None, None) => new.add_sub(old, net, on, off),
    }
}

impl NnueState {
    /// The accumulators of `pos`, computed from scratch, with room for `len`
    /// plies.
    pub(crate) fn new(net: Arc<Network>, pos: &Position, len: usize) -> NnueState {
        let stack = match &*net {
            Network::H128(n) => Stack::H128(Plies::new(n, pos, len)),
            Network::H256(n) => Stack::H256(Plies::new(n, pos, len)),
            Network::H512(n) => Stack::H512(Plies::new(n, pos, len)),
            Network::H768(n) => Stack::H768(Plies::new(n, pos, len)),
        };
        NnueState {
            net,
            stack,
            top: 0,
            stats: RebuildStats::default(),
        }
    }

    /// What the refresh cache did since this state was made.
    pub(crate) fn rebuild_stats(&self) -> RebuildStats {
        self.stats
    }

    /// Spoil every cached accumulator by one, so the next rebuild from any
    /// entry is wrong: a negative control for the tests, which must then see
    /// the difference. Only with the `stats` feature, so release builds
    /// don't carry it.
    #[cfg(feature = "stats")]
    pub(crate) fn corrupt_refresh_cache(&mut self) {
        fn spoil<const H: usize>(p: &mut Plies<H>) {
            for entry in p.cache.iter_mut().flatten() {
                entry.acc.0[0] = entry.acc.0[0].wrapping_add(1);
            }
        }
        match &mut self.stack {
            Stack::H128(p) => spoil(p),
            Stack::H256(p) => spoil(p),
            Stack::H512(p) => spoil(p),
            Stack::H768(p) => spoil(p),
        }
    }

    /// The evaluation for the side to move, with `pieces` pieces on the
    /// board.
    #[inline]
    pub(crate) fn evaluate(&self, stm: Color, pieces: u32) -> i32 {
        let w = usize::from(stm != Color::White);
        let t = self.top;
        match (&*self.net, &self.stack) {
            (Network::H128(n), Stack::H128(s)) => {
                n.evaluate(&s.stack[t][w], &s.stack[t][1 - w], pieces)
            }
            (Network::H256(n), Stack::H256(s)) => {
                n.evaluate(&s.stack[t][w], &s.stack[t][1 - w], pieces)
            }
            (Network::H512(n), Stack::H512(s)) => {
                n.evaluate(&s.stack[t][w], &s.stack[t][1 - w], pieces)
            }
            (Network::H768(n), Stack::H768(s)) => {
                n.evaluate(&s.stack[t][w], &s.stack[t][1 - w], pieces)
            }
            _ => unreachable!("the stack has the network's width"),
        }
    }

    /// Push the accumulators after a move; `pos` is the position after it.
    #[inline]
    pub(crate) fn push(&mut self, d: &MoveDelta, pos: &Position) {
        let (top, stats) = (&mut self.top, &mut self.stats);
        match (&*self.net, &mut self.stack) {
            (Network::H128(n), Stack::H128(s)) => push_move(n, s, top, stats, d, pos),
            (Network::H256(n), Stack::H256(s)) => push_move(n, s, top, stats, d, pos),
            (Network::H512(n), Stack::H512(s)) => push_move(n, s, top, stats, d, pos),
            (Network::H768(n), Stack::H768(s)) => push_move(n, s, top, stats, d, pos),
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
            Stack::H128(s) => s.stack[0] = s.stack[self.top],
            Stack::H256(s) => s.stack[0] = s.stack[self.top],
            Stack::H512(s) => s.stack[0] = s.stack[self.top],
            Stack::H768(s) => s.stack[0] = s.stack[self.top],
        }
        self.top = 0;
    }

    /// The current accumulators, `[white's perspective, black's]`, as plain
    /// values (for tests).
    pub(crate) fn values(&self) -> [Vec<i16>; 2] {
        fn at<const H: usize>(p: &Plies<H>, top: usize) -> [Vec<i16>; 2] {
            [p.stack[top][0].0.to_vec(), p.stack[top][1].0.to_vec()]
        }
        match &self.stack {
            Stack::H128(s) => at(s, self.top),
            Stack::H256(s) => at(s, self.top),
            Stack::H512(s) => at(s, self.top),
            Stack::H768(s) => at(s, self.top),
        }
    }
}
