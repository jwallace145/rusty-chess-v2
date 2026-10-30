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
    Accumulator, FILE_BYTES, HIDDEN, INPUTS, MAX_OUTPUT_WEIGHT, Network, QA, QB, SCALE,
    default_network,
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

/// A random network with weights of the size a trained net has.
fn random_net(seed: u64) -> Network {
    let mut rng = Rng(seed | 1);
    let mut fw: Box<[[i16; HIDDEN]; INPUTS]> = vec![[0i16; HIDDEN]; INPUTS]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!());
    for row in fw.iter_mut() {
        for w in row.iter_mut() {
            *w = rng.range(-40, 40);
        }
    }
    let mut fb = [0i16; HIDDEN];
    fb.iter_mut().for_each(|b| *b = rng.range(-40, 120));
    let mut ow = [0i16; 2 * HIDDEN];
    ow.iter_mut().for_each(|w| *w = rng.range(-127, 127));
    Network::from_parts(fw, fb, ow, rng.range(-2000, 2000))
}

fn board(fen: &str, net: &Arc<Network>) -> Board {
    let mut b = Board::from_position(Position::from_fen(fen).unwrap());
    b.set_network(Some(net.clone()));
    b
}

/// The accumulators recomputed from scratch for the board's position.
fn fresh(b: &Board, net: &Network) -> [rusty_chess_search::nnue::Accumulator; 2] {
    [
        net.refresh(b.position(), Color::White),
        net.refresh(b.position(), Color::Black),
    ]
}

#[test]
fn a_network_file_round_trips_and_wrong_sizes_are_refused() {
    let net = random_net(1);
    let bytes = net.to_bytes();
    assert_eq!(bytes.len(), FILE_BYTES);
    let back = Network::from_bytes(&bytes).unwrap();
    assert_eq!(back.to_bytes(), bytes);
    // Padding to a multiple of 64 bytes is accepted.
    let mut padded = bytes.clone();
    padded.resize(FILE_BYTES.div_ceil(64) * 64, 0);
    assert!(Network::from_bytes(&padded).is_ok());
    assert!(Network::from_bytes(&bytes[..FILE_BYTES - 2]).is_err());
    let mut long = bytes;
    long.resize(FILE_BYTES + 64, 0);
    assert!(
        Network::from_bytes(&long).is_err(),
        "a different architecture"
    );
}

#[test]
fn the_output_is_the_bias_scaled_when_nothing_activates() {
    // All-zero hidden weights and bias: every activation is 0, so the output
    // is the bias times SCALE over QA · QB.
    let fw: Box<[[i16; HIDDEN]; INPUTS]> = vec![[0i16; HIDDEN]; INPUTS]
        .into_boxed_slice()
        .try_into()
        .unwrap_or_else(|_| unreachable!());
    let net = Arc::new(Network::from_parts(fw, [0; HIDDEN], [7; 2 * HIDDEN], 6_400));
    let b = board(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &net,
    );
    assert_eq!(b.evaluate(), 6_400 * SCALE / (QA * QB));
}

#[test]
fn make_and_unmake_keep_the_accumulators_equal_to_a_fresh_computation() {
    // Random walks from positions that reach castling, en passant,
    // promotion and captures, checked at every ply going down and up.
    let net = Arc::new(random_net(7));
    for (i, fen) in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2P5/8/3pP3/8/8/5p2/4K2k w - d6 0 1",
        "r3k2r/1P6/8/8/8/8/6p1/R3K2R b KQkq - 0 1",
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
    let net = Arc::new(random_net(11));
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

#[test]
fn the_evaluation_is_the_same_for_a_mirrored_position() {
    // Perspective features make a net colour-symmetric by construction.
    let net = Arc::new(random_net(3));
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
    let with_net = run(Some(Arc::new(random_net(5))));
    let mut list = MoveList::new();
    Position::from_fen(fen).unwrap().legal_moves(&mut list);
    assert!(list.as_slice().contains(&with_net.best));
    assert_ne!(
        with_net.nodes, plain.nodes,
        "the network evaluated the search"
    );
}

#[test]
fn the_embedded_net_loads_and_judges_material() {
    // A sanity check on the trained net itself; its accuracy against the
    // trainer is `nnue-eval --check`'s job (docs/training.md).
    let net = default_network();
    let level = board(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &net,
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
        let e = board(fen, &net).evaluate();
        assert!(
            e * sign > 500,
            "a queen up is winning for its side: {fen} {e}"
        );
    }
}

/// The output as first written: screlu summed in i64. The engine's i32 form
/// must give exactly this, or the bench signature would move.
fn reference_output(net_bytes: &[u8], us: &Accumulator, them: &Accumulator) -> i32 {
    let words: Vec<i16> = net_bytes
        .chunks_exact(2)
        .map(|c| i16::from_le_bytes([c[0], c[1]]))
        .collect();
    let ow = &words[INPUTS * HIDDEN + HIDDEN..INPUTS * HIDDEN + 3 * HIDDEN];
    let bias = words[INPUTS * HIDDEN + 3 * HIDDEN];
    let screlu = |x: i16| i64::from(x.clamp(0, QA as i16)).pow(2);
    let mut sum = 0i64;
    for i in 0..HIDDEN {
        sum += screlu(us.0[i]) * i64::from(ow[i]);
        sum += screlu(them.0[i]) * i64::from(ow[HIDDEN + i]);
    }
    ((sum / i64::from(QA) + i64::from(bias)) * i64::from(SCALE) / i64::from(QA * QB)) as i32
}

#[test]
fn the_output_is_exact_at_the_extremes_and_on_random_accumulators() {
    let mut rng = Rng(11);
    // Every hidden value clipped to QA and every output weight at the bound:
    // the largest sum a loadable net can produce, within a few percent of
    // i32's range.
    let big = |w: i16| {
        let fw: Box<[[i16; HIDDEN]; INPUTS]> = vec![[0i16; HIDDEN]; INPUTS]
            .into_boxed_slice()
            .try_into()
            .unwrap_or_else(|_| unreachable!());
        Network::from_parts(fw, [0; HIDDEN], [w; 2 * HIDDEN], 0)
    };
    for net in [
        big(MAX_OUTPUT_WEIGHT),
        big(-MAX_OUTPUT_WEIGHT),
        random_net(5),
        Network::from_bytes(&default_network().to_bytes()).unwrap(),
    ] {
        let bytes = net.to_bytes();
        let full = Accumulator([i16::MAX; HIDDEN]);
        assert_eq!(
            net.evaluate(&full, &full),
            reference_output(&bytes, &full, &full)
        );
        for _ in 0..2000 {
            let mut us = Accumulator([0; HIDDEN]);
            let mut them = Accumulator([0; HIDDEN]);
            us.0.iter_mut().for_each(|x| *x = rng.range(-600, 600));
            them.0.iter_mut().for_each(|x| *x = rng.range(-600, 600));
            assert_eq!(
                net.evaluate(&us, &them),
                reference_output(&bytes, &us, &them)
            );
        }
    }
}

#[test]
fn a_net_with_an_output_weight_beyond_the_bound_is_refused() {
    let mut bytes = random_net(7).to_bytes();
    let first_output = 2 * (INPUTS * HIDDEN + HIDDEN);
    bytes[first_output..first_output + 2].copy_from_slice(&128i16.to_le_bytes());
    let err = Network::from_bytes(&bytes).err().expect("refused");
    assert!(err.contains("beyond ±127"), "{err}");
}
