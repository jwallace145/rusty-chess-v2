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

#[test]
fn killer_moves_produce_cutoffs_on_a_real_search() {
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    // Depth 7: pruning removes the shallow nodes where quiet moves cut off.
    s.search(&mut b, &depth(7), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        let killers = summary.cutoffs_by_stage[rusty_chess_search::stats::Stage::Killers as usize];
        assert!(
            killers > 0,
            "the killer stage produced no cutoffs: {:?}",
            summary.cutoffs_by_stage
        );
    }
}

#[test]
fn aspiration_windows_keep_results_exact_and_report_researches() {
    // Mates and draws must survive the narrow windows.
    let r = search(
        "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
        depth(6),
    );
    assert_eq!(
        (r.best.to_string(), mate_in(r.score)),
        ("d5f6".into(), Some(2))
    );
    let r = search("7k/8/8/8/8/8/8/R6K w - - 99 80", depth(6));
    assert_eq!(r.score, 0);
    // One record per depth even when a window fails and is re-searched.
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    let r = s.search(&mut b, &depth(7), &mut |_| {});
    assert_eq!(r.iterations.len(), 7);
    assert!(r.iterations.windows(2).all(|w| w[1].nodes > w[0].nodes));
    // The start position's score moves between depths 6 and 7 by more than
    // the window, so that iteration must have been re-searched.
    let mut b = board("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    let r = s.search(&mut b, &depth(7), &mut |_| {});
    assert_eq!(r.iterations.len(), 7);
    if let Some(summary) = s.stats().summary() {
        assert!(
            summary.aspiration_researches > 0,
            "{:?}",
            summary.aspiration_researches
        );
    }
}

#[test]
fn null_move_pruning_cuts_nodes_but_never_in_check_or_pawn_endings() {
    // Mates, draws and determinism must survive pruning.
    let r = search(
        "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
        depth(6),
    );
    assert_eq!(
        (r.best.to_string(), mate_in(r.score)),
        ("d5f6".into(), Some(2))
    );
    let r = search("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", depth(4));
    assert_eq!(
        (r.best.to_string(), mate_in(r.score)),
        ("a1a8".into(), Some(1))
    );
    // A busy middlegame prunes with null moves.
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.search(&mut b, &depth(6), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert!(
            summary.null_move_cutoffs > 0,
            "no null-move cutoffs in a middlegame"
        );
    }
    // A pawn ending never does: zugzwang would make the null move lie.
    let mut b = board("8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.search(&mut b, &depth(8), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert_eq!(summary.null_move_cutoffs, 0, "null moves in a pawn ending");
    }
}

#[test]
fn reverse_futility_pruning_prunes_but_keeps_mates_and_mate_threats() {
    // Mates stay exact through the pruned depths.
    for (fen, best, mate) in [
        (
            "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
            "d5f6",
            2,
        ),
        ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", "a1a8", 1),
    ] {
        let r = search(fen, depth(6));
        assert_eq!(
            (r.best.to_string(), mate_in(r.score)),
            (best.into(), Some(mate)),
            "{fen}"
        );
    }
    // A queen up, but Re1 mates next move: the static evaluation says
    // "winning" at every depth, and the threat must still be parried.
    let r = search("4r1k1/Q4ppp/8/8/8/8/5PPP/6K1 w - - 0 1", depth(6));
    assert!(
        [
            "g2g3", "h2h3", "a7e3", "a7e7", "a7a1", "a7b8", "a7f2", "g1f1"
        ]
        .contains(&r.best.to_string().as_str()),
        "white must parry Re1#: {} ({})",
        r.best,
        r.pv
    );
    assert!(r.score > 300 && mate_in(r.score).is_none(), "{}", r.score);
    // A busy middlegame prunes.
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.search(&mut b, &depth(6), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert!(summary.rfp_cutoffs > 0, "no reverse futility cutoffs");
    }
}

#[test]
fn late_move_pruning_skips_quiets_but_keeps_mates_and_defences() {
    for (fen, best, mate) in [
        (
            "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
            "d5f6",
            2,
        ),
        ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", "a1a8", 1),
        // Mate in three whose key move is a quiet rook move (Ra6), late in
        // the ordering, through the pruned depths.
        ("r5rk/5p1p/5R2/4B3/8/8/7P/7K w - - 0 1", "f6a6", 3),
    ] {
        let r = search(fen, depth(8));
        assert_eq!(
            (r.best.to_string(), mate_in(r.score)),
            (best.into(), Some(mate)),
            "{fen}"
        );
    }
    // The only defence to a mate threat may be a late quiet move.
    let r = search("4r1k1/Q4ppp/8/8/8/8/5PPP/6K1 w - - 0 1", depth(6));
    assert!(r.score > 300 && mate_in(r.score).is_none(), "{}", r.score);
    // A busy middlegame skips late quiet moves.
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.search(&mut b, &depth(6), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert!(summary.lmp_skips > 0, "no late move pruning skips");
    }
}

#[test]
fn null_move_pruning_does_not_lose_a_known_zugzwang() {
    // White to move; every move loses the rook except Rf1 (Rh1 by the rook's path); the king
    // on h1 is stalemated apart from pawn moves. With naive null-move pruning white would
    // "pass" and misjudge the position. Depth 8 must still find a move that keeps the rook.
    let r = search("8/8/p1p5/1p5p/1P5p/8/PPP2K1p/4R1rk w - - 0 1", depth(8));
    assert!(
        r.best.to_string().starts_with("e1"),
        "white must move the rook: {} ({})",
        r.best,
        r.pv
    );
}

#[test]
fn late_move_reductions_reduce_and_research_without_losing_mates() {
    for (fen, best, mate) in [
        (
            "r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1",
            "d5f6",
            2,
        ),
        ("6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1", "a1a8", 1),
    ] {
        let r = search(fen, depth(6));
        assert_eq!(
            (r.best.to_string(), mate_in(r.score)),
            (best.into(), Some(mate)),
            "{fen}"
        );
    }
    // Quiet moves late in the list are reduced, and some of them are re-searched.
    let mut b = board("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.search(&mut b, &depth(7), &mut |_| {});
    if let Some(summary) = s.stats().summary() {
        assert!(summary.lmr_reductions > 0, "no reductions");
        assert!(summary.lmr_researches > 0, "no re-searches");
        assert!(
            summary.lmr_researches < summary.lmr_reductions,
            "most reductions must stand"
        );
    }
    // The reduction table is monotone in depth and move index.
    use rusty_chess_search::search::lmr_reduction;
    assert_eq!(lmr_reduction(3, 2), lmr_reduction(3, 2));
    assert!(lmr_reduction(12, 30) >= lmr_reduction(6, 30));
    assert!(lmr_reduction(6, 40) >= lmr_reduction(6, 4));
    assert!(lmr_reduction(3, 3) >= 1 && lmr_reduction(20, 60) <= 6);
}
