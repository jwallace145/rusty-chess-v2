//! The search: mates, draws, limits, determinism and its telemetry.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{Limits, MATE, SearchResult, Searcher, mate_in};

fn board(fen: &str) -> Board {
    Board::from_position(Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}")))
}

fn search(fen: &str, limits: Limits) -> SearchResult {
    let mut b = board(fen);
    Searcher::new(Arc::new(AtomicBool::new(false))).search(&mut b, &limits, &mut |_| {})
}

fn depth(d: u32) -> Limits {
    Limits {
        depth: Some(d),
        ..Limits::default()
    }
}

#[test]
fn finds_mate_in_one() {
    let r = search("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", depth(2));
    assert_eq!(r.best.to_string(), "a1a8");
    assert_eq!(r.score, MATE - 1);
    assert_eq!(mate_in(r.score), Some(1));
}

#[test]
fn finds_mate_in_two() {
    let r = search(
        "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
        depth(4),
    );
    assert_eq!(r.best.to_string(), "d5f6");
    assert_eq!(r.score, MATE - 3);
    assert_eq!(mate_in(r.score), Some(2));
    assert!(r.pv.starts_with("d5f6 "), "{}", r.pv);
}

#[test]
fn being_mated_scores_negative_mate_and_prefers_the_longest_defence() {
    // Black has only Kh7, after which Rh1 is mate.
    let r = search("7k/5K2/8/8/8/8/8/6R1 b - - 0 1", depth(3));
    assert_eq!(mate_in(r.score), Some(-1));
    assert!(r.score < 0 && r.score == -(MATE - 2), "{}", r.score);
}

#[test]
fn stalemate_and_checkmate_at_the_root() {
    let r = search("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", depth(3));
    assert!(r.best.to_string() == "0000", "{}", r.best);
    assert_eq!(r.score, 0);
    let r = search("7k/6Q1/6K1/8/8/8/8/8 b - - 0 1", depth(3));
    assert_eq!(r.best.to_string(), "0000");
    assert_eq!(r.score, -MATE);
}

#[test]
fn a_repetition_is_a_draw_and_can_be_the_best_defence() {
    // Black is a queen down; returning the knight to g8 repeats the start
    // position already in the history, which scores 0 and beats everything.
    let mut b = board("rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    for uci in ["g1f3", "g8f6", "f3g1"] {
        let m = b.position().parse_uci_move(uci).unwrap();
        b.make_move(m);
    }
    b.set_root();
    let r = Searcher::new(Arc::new(AtomicBool::new(false))).search(&mut b, &depth(3), &mut |_| {});
    assert_eq!(r.best.to_string(), "f6g8");
    assert_eq!(r.score, 0);
}

#[test]
fn the_fifty_move_rule_is_a_draw() {
    // White is a rook up but any quiet move hits move fifty; a capture is not available.
    let r = search("7k/8/8/8/8/8/8/R6K w - - 99 80", depth(2));
    assert_eq!(r.score, 0, "{}", r.pv);
}

#[test]
fn quiescence_sees_the_recapture() {
    // Rxd5 wins a queen only until exd5; at depth 1 the quiescence search must see that.
    let r = search("4k3/8/4p3/3q4/8/8/8/3R2K1 w - - 0 1", depth(1));
    assert!(r.score < 400, "{} via {}", r.score, r.pv);
    assert!(r.qnodes > 0);
}

#[test]
fn depth_limited_searches_are_deterministic_and_report_every_iteration() {
    let a = search(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth(4),
    );
    let b = search(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        depth(4),
    );
    assert_eq!(
        (a.nodes, a.qnodes, a.best, a.score),
        (b.nodes, b.qnodes, b.best, b.score)
    );
    assert_eq!(a.depth, 4);
    assert_eq!(a.iterations.len(), 4);
    for (i, it) in a.iterations.iter().enumerate() {
        assert_eq!(it.depth as usize, i + 1);
        assert_eq!(it.v, 1);
        assert!(it.pv.split(' ').count() >= 1);
        if i > 0 {
            assert!(it.nodes > a.iterations[i - 1].nodes);
            assert!(it.ebf.is_some_and(|e| e > 1.0), "{:?}", it.ebf);
        } else {
            assert!(it.ebf.is_none());
        }
    }
    let last = a.iterations.last().unwrap();
    assert_eq!(last.nodes, a.nodes);
    assert_eq!(last.score_cp, a.score);
    assert!(last.pv.starts_with(&a.best.to_string()));
    assert!(a.pv.starts_with(&a.best.to_string()));
    assert_eq!(
        a.iterations.iter().all(|i| i.fh_first.is_some()),
        cfg!(feature = "stats"),
        "fail-high-first is reported exactly when the stats feature is on"
    );
}

#[test]
fn node_and_time_limits_and_the_stop_flag_are_honoured() {
    let r = search(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        Limits {
            nodes: Some(20_000),
            ..Limits::default()
        },
    );
    assert!(
        r.nodes + r.qnodes <= 20_000 + 2048,
        "{}",
        r.nodes + r.qnodes
    );
    assert!(r.depth >= 1 && r.best.to_string() != "0000");

    let started = Instant::now();
    let r = search(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        Limits {
            movetime: Some(Duration::from_millis(100)),
            ..Limits::default()
        },
    );
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "{:?}",
        started.elapsed()
    );
    assert!(r.best.to_string() != "0000");

    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let handle = std::thread::spawn(move || {
        let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        Searcher::new(flag).search(
            &mut b,
            &Limits {
                infinite: true,
                ..Limits::default()
            },
            &mut |_| {},
        )
    });
    std::thread::sleep(Duration::from_millis(150));
    stop.store(true, Ordering::Relaxed);
    let r = handle.join().unwrap();
    assert!(r.best.to_string() != "0000");
}

#[test]
fn clock_based_limits_leave_time_on_the_clock() {
    let started = Instant::now();
    let r = search(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        Limits {
            time: Some(Duration::from_millis(1000)),
            inc: Some(Duration::from_millis(0)),
            ..Limits::default()
        },
    );
    assert!(
        started.elapsed() < Duration::from_millis(400),
        "{:?}",
        started.elapsed()
    );
    assert!(r.depth >= 1); // a debug build may not finish depth 2 inside a 1s/30 budget
}

#[test]
fn the_search_leaves_the_board_as_it_found_it() {
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let before = b.position().clone();
    Searcher::new(Arc::new(AtomicBool::new(false))).search(&mut b, &depth(4), &mut |_| {});
    assert_eq!(b.position(), &before);
    assert_eq!(b.ply_from_root(), 0);
}

#[test]
fn stats_counters_match_the_result_when_enabled() {
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    let r = s.search(&mut b, &depth(3), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert_eq!(summary.nodes, r.nodes);
        assert_eq!(summary.qnodes, r.qnodes);
        assert!(summary.fail_highs > 0);
        assert!(summary.fh_first > 0.0 && summary.fh_first <= 1.0);
    }
}
