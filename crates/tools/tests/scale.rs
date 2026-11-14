//! `scale`: removing a piece from a FEN, and what the evaluations say a
//! piece is worth.

use braveheart_core::types::Square;
use braveheart_search::nnue::default_network;
use braveheart_tools::scale::{evaluate, piece_values, summarize, without_piece};

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

fn openings() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests/openings/openings.epd"
    );
    std::fs::read_to_string(path)
        .expect("the committed openings")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn removing_a_piece_rewrites_the_placement_and_drops_the_rights() {
    braveheart_core::attacks::init();
    // The queen's knight, a corner rook, a pawn in the middle of a row.
    assert_eq!(
        without_piece(START, Square::B1).as_deref(),
        Some("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/R1BQKBNR w - - 0 1")
    );
    assert_eq!(
        without_piece(START, Square::H8).as_deref(),
        Some("rnbqkbn1/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w - - 0 1")
    );
    assert_eq!(
        without_piece(START, Square::E2).as_deref(),
        Some("rnbqkbnr/pppppppp/8/8/8/8/PPPP1PPP/RNBQKBNR w - - 0 1")
    );
    // A lone piece on a row leaves an empty row; the side to move is kept.
    assert_eq!(
        without_piece("4k3/8/8/3q4/8/8/8/4K3 b - - 3 40", Square::D5).as_deref(),
        Some("4k3/8/8/8/8/8/8/4K3 b - - 0 1")
    );
    // An empty square has nothing to remove, and a bad placement is refused.
    assert_eq!(without_piece(START, Square::E4), None);
    assert_eq!(
        without_piece("rnbqkbnr/pppppppp w - - 0 1", Square::A1),
        None
    );
    assert_eq!(without_piece("9/8/8/8/8/8/8/8 w - - 0 1", Square::A1), None);
}

#[test]
fn the_hand_written_evaluation_reads_its_own_piece_values() {
    braveheart_core::attacks::init();
    // The control: PeSTO's tables are known (a pawn about 80 to 95, a knight
    // 280 to 340, a bishop 300 to 365, a rook 480 to 510, a queen 940 to
    // 1,025, plus or minus each square's bonus), so the measurement must
    // land on them.
    let fens = openings();
    let values = piece_values(fens.iter().map(String::as_str), None);
    let mean = |i: usize| summarize(&values[i]).expect("pieces of every kind").mean;
    assert!((60.0..120.0).contains(&mean(0)), "pawn {}", mean(0));
    assert!((280.0..360.0).contains(&mean(1)), "knight {}", mean(1));
    assert!((310.0..390.0).contains(&mean(2)), "bishop {}", mean(2));
    assert!((420.0..520.0).contains(&mean(3)), "rook {}", mean(3));
    assert!((950.0..1060.0).contains(&mean(4)), "queen {}", mean(4));
    // Every piece on the board is measured once: 300 openings of about 31
    // pieces each, less the kings and the few positions in check.
    let samples: usize = values.iter().map(Vec::len).sum();
    assert!(samples > 8_000, "{samples}");
}

#[test]
fn the_embedded_net_values_material_in_the_right_order_and_on_its_own_scale() {
    braveheart_core::attacks::init();
    let net = default_network();
    let fens = openings();
    let values = piece_values(fens.iter().map(String::as_str), Some(&net));
    let mean: Vec<f64> = values
        .iter()
        .map(|v| summarize(v).expect("pieces of every kind").mean)
        .collect();
    // Whatever the net's units, a queen is worth more than a rook, a rook
    // more than a minor piece and a minor piece more than a pawn.
    assert!(
        mean[0] < mean[1] && mean[1] < mean[3] && mean[3] < mean[4],
        "{mean:?}"
    );
    assert!(mean[0] < mean[2] && mean[2] < mean[3], "{mean:?}");
    // And its units are not the hand-written evaluation's: a net trained on
    // win probability values a pawn at well over 100. This is the fact the
    // search's margins have to be read against.
    assert!(mean[0] > 120.0, "pawn {}", mean[0]);
}

#[test]
fn a_piece_is_valued_from_its_owners_side_whoever_is_to_move() {
    braveheart_core::attacks::init();
    // The same position with either side to move: white's extra queen is
    // worth a positive amount both times.
    for fen in [
        "4k3/8/8/8/8/8/3Q4/4K3 w - - 0 1",
        "4k3/8/8/8/8/8/3Q4/4K3 b - - 0 1",
    ] {
        let values = piece_values([fen], None);
        assert_eq!(values[4].len(), 1, "{fen}");
        assert!(values[4][0] > 800, "{fen}: {}", values[4][0]);
    }
    // In check there is no evaluation to compare.
    assert_eq!(evaluate("4k3/8/8/8/8/8/4Q3/4K3 b - - 0 1", None), None);
    assert!(summarize(&[]).is_none());
    let s = summarize(&[10, 30, 20]).expect("three values");
    assert_eq!((s.samples, s.median, s.p10, s.p90), (3, 20, 10, 20));
    assert!((s.mean - 20.0).abs() < 1e-9);
}
