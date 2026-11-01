//! NNUE: the file format, the accumulator kept through make and unmake, and
//! the evaluation, on synthetic networks (no trained net is needed to check
//! the arithmetic and the bookkeeping).

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::Color;
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::{
    DEFAULT_NET_ID, INPUTS, KING_BUCKET_LAYOUT, KING_BUCKETS, MAX_OUTPUT_WEIGHT, NETS, Network,
    OUTPUT_BUCKETS, QA, QB, SCALE, WIDTHS, bucketed_file_bytes, default_network, file_bytes,
    king_view, net_file_bytes, network_by_id, output_bucket,
};
use rusty_chess_search::search::{Limits, Searcher};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn range(&mut self, lo: i16, hi: i16) -> i16 {
        lo + (self.next() % (hi - lo + 1) as u64) as i16
    }
}

/// Every (width, output buckets, king buckets) the engine reads.
fn shapes() -> impl Iterator<Item = (usize, usize, usize)> {
    WIDTHS.iter().flat_map(|&h| {
        OUTPUT_BUCKETS
            .iter()
            .flat_map(move |&b| KING_BUCKETS.iter().map(move |&k| (h, b, k)))
    })
}

/// A network of width `hidden`, `buckets` output buckets and `king` king
/// buckets from its weights in file order: feature weights (king bucket 0's
/// first), feature bias, each output bucket's weights (ours, theirs), the
/// output biases.
fn net_from(
    hidden: usize,
    buckets: usize,
    king: usize,
    mut word: impl FnMut(usize) -> i16,
) -> Network {
    let words = net_file_bytes(hidden, buckets, king) / 2;
    let bytes: Vec<u8> = (0..words).flat_map(|i| word(i).to_le_bytes()).collect();
    Network::from_bytes(&bytes).unwrap()
}

/// Where a net's sections start, in words: feature bias, output weights,
/// output biases.
fn sections(hidden: usize, buckets: usize, king: usize) -> (usize, usize, usize) {
    let fb = INPUTS * king * hidden;
    let ow = fb + hidden;
    (fb, ow, ow + buckets * 2 * hidden)
}

/// A random network with weights of the size a trained net has.
fn random_net(seed: u64, hidden: usize, buckets: usize, king: usize) -> Network {
    let mut rng = Rng(seed | 1);
    let (fb, ow, ob) = sections(hidden, buckets, king);
    net_from(hidden, buckets, king, |i| match i {
        _ if i < fb => rng.range(-40, 40),
        _ if i < ow => rng.range(-40, 120),
        _ if i < ob => rng.range(-127, 127),
        _ => rng.range(-2000, 2000),
    })
}

/// A network with zero hidden weights and biases, every output weight `w`
/// and every output bias `bias`.
fn flat_net(hidden: usize, buckets: usize, king: usize, w: i16, bias: i16) -> Network {
    let (_, ow, ob) = sections(hidden, buckets, king);
    net_from(hidden, buckets, king, |i| match i {
        _ if i < ow => 0,
        _ if i < ob => w,
        _ => bias,
    })
}

/// A FEN mirrored left to right (files a ↔ h), without castling or en passant.
fn mirror_fen(fen: &str) -> String {
    let mut parts = fen.split(' ');
    let board: Vec<String> = parts
        .next()
        .unwrap()
        .split('/')
        .map(|rank| rank.chars().rev().collect())
        .collect();
    let side = parts.next().unwrap();
    format!("{} {side} - - 0 1", board.join("/"))
}

fn board(fen: &str, net: &Arc<Network>) -> Board {
    let mut b = Board::from_position(Position::from_fen(fen).unwrap());
    b.set_network(Some(net.clone()));
    b
}

/// The accumulators recomputed from scratch for the board's position.
fn fresh(b: &Board, net: &Network) -> [Vec<i16>; 2] {
    [
        net.refresh_values(b.position(), Color::White),
        net.refresh_values(b.position(), Color::Black),
    ]
}

#[test]
fn a_network_file_round_trips_and_wrong_sizes_are_refused() {
    for (h, buckets, king) in shapes() {
        let size = net_file_bytes(h, buckets, king);
        let net = random_net(1, h, buckets, king);
        assert_eq!(
            (net.hidden(), net.buckets(), net.king_buckets()),
            (h, buckets, king)
        );
        let bytes = net.to_bytes();
        assert_eq!(bytes.len(), size);
        let back = Network::from_bytes(&bytes).unwrap();
        assert_eq!(back.to_bytes(), bytes);
        // Padding to a multiple of 64 bytes is accepted.
        let mut padded = bytes.clone();
        padded.resize(size.div_ceil(64) * 64, 0);
        let back = Network::from_bytes(&padded).unwrap();
        assert_eq!(
            (back.hidden(), back.buckets(), back.king_buckets()),
            (h, buckets, king)
        );
        assert!(Network::from_bytes(&bytes[..size - 2]).is_err());
        let mut long = bytes;
        long.resize(size + 64, 0);
        assert!(
            Network::from_bytes(&long).is_err(),
            "a different architecture"
        );
    }
}

#[test]
fn the_output_is_the_bias_scaled_when_nothing_activates() {
    // All-zero hidden weights and bias: every activation is 0, so the output
    // is the bias times SCALE over QA · QB.
    for (h, buckets, king) in shapes() {
        let net = Arc::new(flat_net(h, buckets, king, 7, 6_400));
        let b = board(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &net,
        );
        assert_eq!(
            b.evaluate(),
            6_400 * SCALE / (QA * QB),
            "width {h}, {buckets} buckets, {king} king buckets"
        );
    }
}

#[test]
fn make_and_unmake_keep_the_accumulators_equal_to_a_fresh_computation() {
    for &h in &WIDTHS {
        for &king in &KING_BUCKETS {
            walks(h, king);
        }
    }
}

fn walks(hidden: usize, king: usize) {
    // Random walks from positions that reach castling, en passant,
    // promotion and captures, checked at every ply going down and up.
    let net = Arc::new(random_net(7, hidden, 1, king));
    // The last two give the kings room to walk across the d/e mirror line
    // and between king buckets.
    for (i, fen) in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2P5/8/3pP3/8/8/5p2/4K2k w - d6 0 1",
        "r3k2r/1P6/8/8/8/8/6p1/R3K2R b KQkq - 0 1",
        "8/2p5/3k4/8/8/4K3/5P2/8 w - - 0 1",
        "4k3/8/8/3n4/8/8/3N4/4K3 b - - 0 1",
    ]
    .iter()
    .enumerate()
    {
        let mut rng = Rng(0x9E37_79B9 + i as u64);
        for _ in 0..20 {
            let mut b = board(fen, &net);
            let mut line = Vec::new();
            for _ in 0..12 {
                let mut list = MoveList::new();
                b.position().legal_moves(&mut list);
                if list.is_empty() {
                    break;
                }
                let m = list.as_slice()[(rng.next() % list.len() as u64) as usize];
                let undo = b.make_move(m);
                assert_eq!(
                    b.accumulators().unwrap(),
                    fresh(&b, &net),
                    "{fen} after {m}"
                );
                line.push((m, undo));
            }
            while let Some((m, undo)) = line.pop() {
                b.unmake_move(m, undo);
                assert_eq!(
                    b.accumulators().unwrap(),
                    fresh(&b, &net),
                    "{fen} undoing {m}"
                );
            }
        }
    }
}

#[test]
fn a_long_game_played_forward_wraps_the_stack_safely() {
    // More plies than the stack holds, played without unmaking, as the UCI
    // `position ... moves` and self-play do.
    for (h, _, king) in shapes().filter(|s| s.1 == 1) {
        let net = Arc::new(random_net(11, h, 1, king));
        let mut b = board(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            &net,
        );
        let shuffle = ["g1f3", "g8f6", "f3g1", "f6g8"];
        for i in 0..400 {
            let m = b.position().parse_uci_move(shuffle[i % 4]).unwrap();
            b.make_move(m);
            assert_eq!(b.accumulators().unwrap(), fresh(&b, &net));
        }
        b.set_root();
        assert_eq!(b.accumulators().unwrap(), fresh(&b, &net));
    }
}

#[test]
fn the_evaluation_is_the_same_for_a_mirrored_position() {
    // Perspective features make a net colour-symmetric by construction.
    for (h, buckets, king) in shapes() {
        mirrored_at(h, buckets, king);
    }
}

fn mirrored_at(hidden: usize, buckets: usize, king: usize) {
    let net = Arc::new(random_net(3, hidden, buckets, king));
    for (fen, mirrored) in [
        (
            "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w - - 4 4",
            "rnbqk2r/pppp1ppp/5n2/2b1p3/4P3/2N2N2/PPPP1PPP/R1BQKB1R b - - 4 4",
        ),
        (
            "8/5k2/3p4/1p1Pp2p/pP2Pp1P/P4P1K/8/8 b - - 0 1",
            "8/8/p4p1k/Pp2pP1p/1P1pP2P/3P4/5K2/8 w - - 0 1",
        ),
    ] {
        assert_eq!(
            board(fen, &net).evaluate(),
            board(mirrored, &net).evaluate(),
            "{fen}"
        );
    }
}

#[test]
fn a_search_with_a_network_runs_and_without_one_is_unchanged() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let limits = Limits {
        depth: Some(6),
        ..Limits::default()
    };
    let run = |net: Option<Arc<Network>>| {
        let mut b = Board::from_position(Position::from_fen(fen).unwrap());
        b.set_network(net);
        let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
        s.search(&mut b, &limits, &mut |_| {})
    };
    let plain = run(None);
    let again = run(None);
    assert_eq!((plain.best, plain.nodes), (again.best, again.nodes));
    let with_net = run(Some(Arc::new(random_net(5, 256, 1, 1))));
    let mut list = MoveList::new();
    Position::from_fen(fen).unwrap().legal_moves(&mut list);
    assert!(list.as_slice().contains(&with_net.best));
    assert_ne!(
        with_net.nodes, plain.nodes,
        "the network evaluated the search"
    );
}

#[test]
fn the_embedded_nets_are_ordered_and_the_last_is_the_default() {
    assert!(
        NETS.windows(2).all(|w| w[0].generation < w[1].generation),
        "one net per generation, oldest first"
    );
    assert_eq!(DEFAULT_NET_ID, NETS[NETS.len() - 1].id);
    assert_eq!(
        default_network().to_bytes(),
        network_by_id(DEFAULT_NET_ID).unwrap().to_bytes()
    );
    assert!(network_by_id("no-such-net").is_none());
}

#[test]
fn every_embedded_net_loads_and_judges_material() {
    // A sanity check on the trained nets themselves; their accuracy against
    // the trainer is `nnue-eval --check`'s job (docs/training.md).
    for embedded in &NETS {
        material_sanity(&network_by_id(embedded.id).unwrap());
    }
}

fn material_sanity(net: &Arc<Network>) {
    let level = board(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        net,
    )
    .evaluate();
    assert!(level.abs() < 100, "the start position is level: {level}");
    for (fen, sign) in [
        (
            "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            1,
        ),
        (
            "rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1",
            -1,
        ),
    ] {
        let e = board(fen, net).evaluate();
        assert!(
            e * sign > 500,
            "a queen up is winning for its side: {fen} {e}"
        );
    }
}

/// The output as first written: screlu summed in i64. The engine's i32 form
/// must give exactly this, or the bench signature would move.
struct Output {
    weights: Vec<i16>,
    bias: i16,
}

/// Bucket `bucket` of a net's output layer, read back from its file bytes.
fn output_layer(net: &Network, bucket: usize) -> Output {
    let h = net.hidden();
    let (_, ow, ob) = sections(h, net.buckets(), net.king_buckets());
    let words: Vec<i16> = net
        .to_bytes()
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let start = ow + bucket * 2 * h;
    Output {
        weights: words[start..start + 2 * h].to_vec(),
        bias: words[ob + bucket],
    }
}

fn reference_output(out: &Output, us: &[i16], them: &[i16]) -> i32 {
    let h = us.len();
    let (ow, bias) = (&out.weights, out.bias);
    let screlu = |x: i16| i64::from(x.clamp(0, QA as i16)).pow(2);
    let mut sum = 0i64;
    for i in 0..h {
        sum += screlu(us[i]) * i64::from(ow[i]);
        sum += screlu(them[i]) * i64::from(ow[h + i]);
    }
    ((sum / i64::from(QA) + i64::from(bias)) * i64::from(SCALE) / i64::from(QA * QB)) as i32
}

#[test]
fn the_output_is_exact_at_the_extremes_and_on_random_accumulators() {
    let mut rng = Rng(11);
    // Every hidden value clipped to QA and every output weight at the bound:
    // the largest sum a loadable net can produce. At 256 neurons the two
    // sides together exceed i32's range (2 · 2,114,092,800); at 512 one side
    // alone does. These are the cases that need the sum in i32 chunks of 256
    // and the chunks added in i64.
    let mut nets = Vec::new();
    // King buckets change only which input columns build the accumulator,
    // never the output sum, so single-view nets cover every output shape.
    for (h, buckets, king) in shapes().filter(|s| s.2 == 1) {
        nets.push(flat_net(h, buckets, king, MAX_OUTPUT_WEIGHT, 0));
        nets.push(flat_net(h, buckets, king, -MAX_OUTPUT_WEIGHT, 0));
        nets.push(random_net(5, h, buckets, king));
    }
    nets.push(Network::from_bytes(&default_network().to_bytes()).unwrap());
    for net in &nets {
        for bucket in 0..net.buckets() {
            exact_in_bucket(net, bucket, &mut rng);
        }
    }
}

/// A piece count that selects `bucket` in `net`.
fn pieces_for(net: &Network, bucket: usize) -> u32 {
    (2..=32)
        .find(|&p| output_bucket(p, net.buckets()) == bucket)
        .expect("every bucket has a piece count")
}

fn exact_in_bucket(net: &Network, bucket: usize, rng: &mut Rng) {
    {
        let h = net.hidden();
        let out = output_layer(net, bucket);
        let pieces = pieces_for(net, bucket);
        let full = vec![i16::MAX; h];
        let expected = reference_output(&out, &full, &full);
        assert_eq!(
            net.evaluate_values(&full, &full, pieces),
            expected,
            "width {h}, bucket {bucket}"
        );
        let side = 255i64 * 255 * i64::from(MAX_OUTPUT_WEIGHT) * h as i64;
        if h >= 256 {
            assert!(2 * side > i64::from(i32::MAX), "both sides are beyond i32");
        }
        if h >= 512 {
            assert!(side > i64::from(i32::MAX), "one side alone is beyond i32");
        }
        // 2000 random cases a net, spread over its buckets.
        for _ in 0..2000 / net.buckets() {
            let us: Vec<i16> = (0..h).map(|_| rng.range(-600, 600)).collect();
            let them: Vec<i16> = (0..h).map(|_| rng.range(-600, 600)).collect();
            assert_eq!(
                net.evaluate_values(&us, &them, pieces),
                reference_output(&out, &us, &them)
            );
        }
    }
}

#[test]
fn a_net_with_an_output_weight_beyond_the_bound_is_refused() {
    for (h, buckets, king) in shapes() {
        let (_, ow, ob) = sections(h, buckets, king);
        // The first output weight, and the last bucket's last one.
        for word in [ow, ob - 1] {
            let mut bytes = random_net(7, h, buckets, king).to_bytes();
            bytes[2 * word..2 * word + 2].copy_from_slice(&128i16.to_le_bytes());
            let err = Network::from_bytes(&bytes).err().expect("refused");
            assert!(err.contains("beyond ±127"), "{err}");
        }
    }
}

#[test]
fn output_buckets_split_the_piece_counts_into_equal_bands() {
    // bullet's MaterialCount<8>: (pieces − 2) / 4.
    let eight: Vec<usize> = (2..=32).map(|p| output_bucket(p, 8)).collect();
    let expected: Vec<usize> = (2..=32usize).map(|p| ((p - 2) / 4).min(7)).collect();
    assert_eq!(eight, expected);
    assert_eq!(
        (
            output_bucket(2, 8),
            output_bucket(5, 8),
            output_bucket(6, 8)
        ),
        (0, 0, 1)
    );
    assert_eq!((output_bucket(29, 8), output_bucket(30, 8)), (6, 7));
    assert_eq!(output_bucket(32, 8), 7);
    assert!((2..=32).all(|p| output_bucket(p, 1) == 0));
}

#[test]
fn a_bucketed_net_uses_the_bucket_of_the_piece_count_on_the_board() {
    // Zero hidden weights and a different output bias per bucket: the
    // evaluation is the bias of the bucket the position falls in.
    for &h in &WIDTHS {
        let (_, _, ob) = sections(h, 8, 1);
        let net = Arc::new(net_from(h, 8, 1, |i| {
            if i < ob {
                0
            } else {
                1_000 * (i - ob + 1) as i16
            }
        }));
        for (fen, pieces) in [
            (
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                32,
            ),
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                32,
            ),
            ("8/5k2/3p4/1p1Pp2p/pP2Pp1P/P4P1K/8/8 b - - 0 1", 14),
            ("8/2P5/8/3pP3/8/8/5p2/4K2k w - d6 0 1", 6),
            ("8/8/8/8/8/8/5k2/4K3 w - - 0 1", 2),
        ] {
            let b = board(fen, &net);
            assert_eq!(b.position().occupied().count(), pieces, "{fen}");
            let bias = 1_000 * (output_bucket(pieces, 8) as i32 + 1);
            assert_eq!(b.evaluate(), bias * SCALE / (QA * QB), "width {h}: {fen}");
        }
    }
}

#[test]
fn single_bucket_files_keep_their_size_and_buckets_are_thousands_of_bytes_apart() {
    for &h in &WIDTHS {
        assert_eq!(file_bytes(h), bucketed_file_bytes(h, 1));
        assert_eq!(file_bytes(h), 2 * (INPUTS * h + h + 2 * h + 1));
    }
    let sizes: Vec<usize> = shapes().map(|(h, b, k)| net_file_bytes(h, b, k)).collect();
    for (i, a) in sizes.iter().enumerate() {
        for b in &sizes[i + 1..] {
            assert!(
                a.abs_diff(*b) >= 64,
                "{a} and {b} could be confused with padding"
            );
        }
    }
    // Every net before net5b8 has one output; net5b8 is the first with 8.
    for n in &NETS {
        let want = if n.generation >= 5 { 8 } else { 1 };
        assert_eq!(network_by_id(n.id).unwrap().buckets(), want, "{}", n.id);
    }
}

#[test]
fn king_views_follow_the_layout_and_mirror_files_e_to_h() {
    // One king bucket is a single view: today's indexing.
    assert!((0..64).all(|sq| king_view(1, sq) == (0, 0)));
    // From its own side: a1 b1 → bucket 0, c1 d1 → 1, rank 2 → 2, above → 3,
    // and files e–h mirror a–d with the file XOR.
    let at = |sq: usize| king_view(4, sq);
    assert_eq!(
        (at(0), at(1), at(2), at(3)),
        ((0, 0), (0, 0), (768, 0), (768, 0))
    );
    assert_eq!(
        (at(4), at(5), at(6), at(7)),
        ((768, 7), (768, 7), (0, 7), (0, 7))
    );
    assert!((8..16).all(|sq| at(sq).0 == 2 * 768));
    assert!((16..64).all(|sq| at(sq).0 == 3 * 768));
    for sq in 0..64 {
        let mirrored = sq ^ 7;
        assert_eq!(
            at(sq).0,
            at(mirrored).0,
            "the layout is symmetric about d/e"
        );
        assert_ne!(
            at(sq).1,
            at(mirrored).1,
            "exactly one of a square and its mirror flips"
        );
    }
    assert_eq!(KING_BUCKET_LAYOUT.iter().max(), Some(&3));
}

#[test]
fn a_king_bucketed_net_evaluates_a_mirrored_position_identically() {
    // With the board mirrored whenever a king is on files e–h, a position and
    // its left-right mirror are the same inputs to a 4-king-bucket net.
    for (h, buckets, _) in shapes().filter(|s| s.2 == 4) {
        let net = Arc::new(random_net(13, h, buckets, 4));
        for fen in [
            "r1bq1rk1/pppp1ppp/2n2n2/2b1p3/2B1P3/3P1N2/PPP2PPP/RNBQ1RK1 w - - 0 1",
            "8/2p5/3k4/8/8/4K3/5P2/8 b - - 0 1",
            "2kr3r/ppp2ppp/8/8/8/8/PPP2PPP/R4RK1 w - - 0 1",
        ] {
            let mirrored = mirror_fen(fen);
            assert_eq!(
                board(fen, &net).evaluate(),
                board(&mirrored, &net).evaluate(),
                "width {h}, {buckets} buckets: {fen} vs {mirrored}"
            );
        }
    }
}

#[test]
fn a_king_move_across_the_mirror_line_rebuilds_its_side_only() {
    // Ke1-d1 crosses the d/e line (white's view changes and is rebuilt);
    // black's accumulator updates incrementally. Both must equal a fresh
    // computation, and the evaluation must match a board set up directly.
    let net = Arc::new(random_net(17, 128, 1, 4));
    let mut b = board("4k3/8/8/8/8/8/8/R3K3 w - - 0 1", &net);
    let m = b.position().parse_uci_move("e1d1").unwrap();
    let undo = b.make_move(m);
    assert_eq!(b.accumulators().unwrap(), fresh(&b, &net));
    assert_eq!(
        b.evaluate(),
        board("4k3/8/8/8/8/8/8/R2K4 b - - 0 1", &net).evaluate()
    );
    b.unmake_move(m, undo);
    assert_eq!(b.accumulators().unwrap(), fresh(&b, &net));
}
