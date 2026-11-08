//! The pawn-structure correction history: its index, its bounds, its update
//! and the bound rule (`docs/correction-history.md`).

use rusty_chess_core::position::Position;
use rusty_chess_search::corrhist::{CorrectionHistory, GRAIN, MAX_CORRECTION};
use rusty_chess_search::search::MATE_BOUND;
use rusty_chess_search::tt::Bound;

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
/// The start position's pawns under different pieces.
const SAME_PAWNS: &str = "rnbqkb1r/pppppppp/5n2/8/8/5N2/PPPPPPPP/RNBQKB1R w KQkq - 2 2";
const SAME_PAWNS_BLACK: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
const OTHER_PAWNS: &str = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap()
}

#[test]
fn an_empty_table_changes_nothing() {
    let t = CorrectionHistory::new();
    for fen in [START, SAME_PAWNS, OTHER_PAWNS] {
        let p = pos(fen);
        assert_eq!(t.correction(&p), 0);
        for raw in [-500, -1, 0, 37, 800] {
            assert_eq!(t.corrected(&p, raw), raw);
        }
    }
}

#[test]
fn the_key_is_the_pawn_structure_and_the_side_to_move_has_its_own_entry() {
    assert_eq!(
        CorrectionHistory::key(&pos(START)),
        CorrectionHistory::key(&pos(SAME_PAWNS)),
        "the same pawns under other pieces"
    );
    assert_ne!(
        CorrectionHistory::key(&pos(START)),
        CorrectionHistory::key(&pos(OTHER_PAWNS)),
        "one pawn moved"
    );

    let mut t = CorrectionHistory::new();
    for _ in 0..50 {
        t.update(&pos(START), 0, 100, 8);
    }
    assert!(t.correction(&pos(START)) > 50, "white to move learned");
    assert_eq!(
        t.correction(&pos(SAME_PAWNS)),
        t.correction(&pos(START)),
        "shared by every position with that structure and side"
    );
    assert_eq!(
        t.correction(&pos(SAME_PAWNS_BLACK)),
        0,
        "black to move has its own entry"
    );
}

#[test]
fn updates_move_towards_the_target_converge_and_stay_within_the_cap() {
    let mut t = CorrectionHistory::new();
    let p = pos(START);
    // Towards +50: the pawn table is monotone and settles within a
    // centipawn; the major-piece table, which learns what the pawn table
    // leaves, rises then fades, so the applied sum settles there too.
    let mut last = 0;
    for _ in 0..300 {
        assert_eq!(t.update(&p, 0, 50, 10), (false, false));
        let c = t.pawn_correction(&p);
        assert!(c >= last, "monotone");
        assert!(c <= 50);
        assert!(
            t.correction(&p) <= 60,
            "the sum never runs far past the error"
        );
        last = c;
    }
    assert!((49..=50).contains(&last), "{last}");
    assert!(
        (48..=52).contains(&t.correction(&p)),
        "{}",
        t.correction(&p)
    );

    // An error beyond the cap is reported, its target is clamped to the cap,
    // and the entry settles just under it (integer division stops a step
    // short) and never beyond.
    let mut t = CorrectionHistory::new();
    assert!(t.update(&p, 0, 1_000, 20).0, "reported as beyond the cap");
    assert!(!t.update(&p, 0, 100, 20).0, "within the cap");
    for _ in 0..400 {
        t.update(&p, 0, 1_000, 20);
    }
    assert!((MAX_CORRECTION - 1..=MAX_CORRECTION).contains(&t.correction(&p)));
    for _ in 0..400 {
        t.update(&p, 0, -1_000, 20);
    }
    assert!((-MAX_CORRECTION..=-(MAX_CORRECTION - 1)).contains(&t.correction(&p)));

    // Shallow updates weigh less than deep ones.
    let (mut shallow, mut deep) = (CorrectionHistory::new(), CorrectionHistory::new());
    shallow.update(&p, 0, 100, 0);
    deep.update(&p, 0, 100, 15);
    assert!(deep.correction(&p) > shallow.correction(&p));
    assert_eq!(GRAIN, 256);
}

#[test]
fn a_bound_is_evidence_only_in_the_direction_of_the_error() {
    // The search's own evaluation was 20. A fail-high at 60 says the position
    // is at least 60: evidence upwards. A fail-high at 10 says nothing.
    assert!(CorrectionHistory::is_evidence(Bound::Exact, -300, 20));
    assert!(CorrectionHistory::is_evidence(Bound::Exact, 300, 20));
    assert!(CorrectionHistory::is_evidence(Bound::Lower, 60, 20));
    assert!(!CorrectionHistory::is_evidence(Bound::Lower, 10, 20));
    assert!(!CorrectionHistory::is_evidence(Bound::Lower, 20, 20));
    assert!(CorrectionHistory::is_evidence(Bound::Upper, -10, 20));
    assert!(!CorrectionHistory::is_evidence(Bound::Upper, 60, 20));
    assert!(!CorrectionHistory::is_evidence(Bound::Upper, 20, 20));
}

#[test]
fn a_corrected_evaluation_never_reaches_the_mate_range() {
    let mut t = CorrectionHistory::new();
    let p = pos(START);
    for _ in 0..400 {
        t.update(&p, 0, 1_000, 20);
    }
    assert!(t.correction(&p) >= MAX_CORRECTION - 1);
    let high = t.corrected(&p, MATE_BOUND - 1);
    assert!((MATE_BOUND - 1..MATE_BOUND).contains(&high));
    for _ in 0..400 {
        t.update(&p, 0, -1_000, 20);
    }
    let low = t.corrected(&p, -(MATE_BOUND - 1));
    assert!((-MATE_BOUND + 1..=-(MATE_BOUND - 1)).contains(&low));
}

/// The start position's pawns and major pieces, with white's h-rook gone:
/// the same pawn key, another major key.
const NO_H_ROOK: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN1 w Qkq - 0 1";

#[test]
fn the_major_key_is_the_rooks_queens_and_kings() {
    assert_eq!(
        CorrectionHistory::major_key(&pos(START)),
        CorrectionHistory::major_key(&pos(SAME_PAWNS)),
        "knights moved: the same majors"
    );
    assert_eq!(
        CorrectionHistory::major_key(&pos(START)),
        CorrectionHistory::major_key(&pos(OTHER_PAWNS)),
        "a pawn moved: the same majors"
    );
    assert_ne!(
        CorrectionHistory::major_key(&pos(START)),
        CorrectionHistory::major_key(&pos(NO_H_ROOK)),
        "a rook gone"
    );
    assert_eq!(
        CorrectionHistory::key(&pos(START)),
        CorrectionHistory::key(&pos(NO_H_ROOK))
    );
}

#[test]
fn the_second_table_learns_what_the_pawn_table_cannot_tell_apart() {
    // Two positions with the same pawns but different major pieces, one
    // evaluated 80 cp too low and the other 80 cp too high. The pawn table
    // sees both and averages to nothing; the major-piece table separates
    // them, so each position gets its own correction.
    let mut t = CorrectionHistory::new();
    let (a, b) = (pos(START), pos(NO_H_ROOK));
    for _ in 0..300 {
        t.update(&a, 0, 80, 10);
        t.update(&b, 0, -80, 10);
    }
    assert!(
        t.pawn_correction(&a).abs() < 20,
        "{}",
        t.pawn_correction(&a)
    );
    assert!(t.correction(&a) >= 60, "{}", t.correction(&a));
    assert!(t.correction(&b) <= -60, "{}", t.correction(&b));
    // The sum is held within the cap even when both tables are at theirs.
    let mut t = CorrectionHistory::new();
    for _ in 0..400 {
        t.update(&a, 0, 1_000, 20);
    }
    assert!(t.pawn_correction(&a) >= MAX_CORRECTION - 1);
    assert!(t.major_correction(&a) >= MAX_CORRECTION - 1);
    assert_eq!(t.correction(&a), MAX_CORRECTION);
}

#[test]
fn clearing_forgets_everything() {
    let mut t = CorrectionHistory::new();
    let p = pos(START);
    for _ in 0..20 {
        t.update(&p, 0, 100, 10);
    }
    assert!(t.correction(&p) > 0);
    t.clear();
    assert_eq!(t.correction(&p), 0);
    assert_eq!(t.correction(&pos(SAME_PAWNS_BLACK)), 0);
}

mod in_search {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    use rusty_chess_core::position::Position;
    use rusty_chess_search::board::Board;
    use rusty_chess_search::nnue::default_network;
    use rusty_chess_search::search::{Limits, SearchResult, Searcher, is_mate_score};

    const QUIET: [&str; 3] = [
        "r1bq1rk1/pp2bppp/2n1pn2/3p4/2PP4/2N1PN2/PP3PPP/R1BQKB1R w KQ - 0 8",
        "8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];

    fn search(s: &mut Searcher, fen: &str, depth: u32) -> SearchResult {
        let mut b = Board::from_position(Position::from_fen(fen).unwrap());
        b.set_network(Some(default_network()));
        let limits = Limits {
            depth: Some(depth),
            ..Limits::default()
        };
        s.search(&mut b, &limits, &mut |_| {})
    }

    #[test]
    fn the_search_stays_deterministic_and_free_of_false_mates_and_the_table_learns() {
        let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
        for fen in QUIET {
            s.clear_hash();
            let a = search(&mut s, fen, 8);
            assert!(!is_mate_score(a.score), "{fen}: {}", a.score);
            // The same search from a cleared searcher repeats exactly: the
            // table is part of the state `clear_hash` resets.
            s.clear_hash();
            let b = search(&mut s, fen, 8);
            assert_eq!(
                (a.nodes, a.qnodes, a.score, a.best),
                (b.nodes, b.qnodes, b.score, b.best)
            );
        }
        // With the counters compiled in, the table was read and written.
        if let Some(summary) = s.stats().summary() {
            let c = summary.correction;
            assert!(c.reads > 0, "{c:?}");
            assert!(
                c.updates_exact + c.updates_lower + c.updates_upper > 0,
                "{c:?}"
            );
            assert!(c.mean_abs_correction <= 128.0, "{c:?}");
        }
    }
}
