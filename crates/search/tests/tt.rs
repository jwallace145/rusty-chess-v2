//! The transposition table: packing, key verification, mate-score
//! adjustment, replacement, aging, and its effect on the search.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusty_chess_core::r#move::{Move, MoveFlag};
use rusty_chess_core::position::Position;
use rusty_chess_core::types::Square;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{Limits, MATE, Searcher, mate_in};
use rusty_chess_search::tt::{Bound, TranspositionTable};

fn mv(from: Square, to: Square) -> Move {
    Move::new(from, to, MoveFlag::Quiet)
}

#[test]
#[allow(clippy::assertions_on_constants)] // layout facts, asserted on purpose
fn entries_are_sixteen_bytes_or_less_and_clusters_fit_a_cache_line() {
    assert!(
        TranspositionTable::ENTRY_BYTES <= 16,
        "{}",
        TranspositionTable::ENTRY_BYTES
    );
    assert_eq!(
        TranspositionTable::CLUSTER_BYTES % 32,
        0,
        "{}",
        TranspositionTable::CLUSTER_BYTES
    );
    assert!(TranspositionTable::CLUSTER_BYTES <= 64);
    assert!(
        TranspositionTable::ENTRIES_PER_CLUSTER >= 2
            && TranspositionTable::ENTRIES_PER_CLUSTER <= 4
    );
}

#[test]
fn size_is_a_power_of_two_of_clusters_and_resizing_clears() {
    let tt = TranspositionTable::new(1);
    assert!(tt.clusters().is_power_of_two());
    assert!(tt.clusters() * TranspositionTable::CLUSTER_BYTES <= 1 << 20);
    assert!(tt.clusters() * TranspositionTable::CLUSTER_BYTES > 1 << 19);
    let mut tt = TranspositionTable::new(1);
    tt.store(
        0xDEAD_BEEF_0000_0001,
        mv(Square::E2, Square::E4),
        12,
        5,
        Bound::Exact,
        0,
    );
    assert!(tt.probe(0xDEAD_BEEF_0000_0001, 0).is_some());
    tt.resize(2);
    assert!(tt.probe(0xDEAD_BEEF_0000_0001, 0).is_none());
    assert!(tt.clusters() * TranspositionTable::CLUSTER_BYTES <= 2 << 20);
    tt.store(
        0xDEAD_BEEF_0000_0001,
        mv(Square::E2, Square::E4),
        12,
        5,
        Bound::Exact,
        0,
    );
    tt.clear();
    assert!(tt.probe(0xDEAD_BEEF_0000_0001, 0).is_none());
}

#[test]
fn store_and_probe_round_trip_with_key_verification() {
    let mut tt = TranspositionTable::new(1);
    let key = 0x0123_4567_89AB_CDEF;
    tt.store(key, mv(Square::G1, Square::F3), -37, 7, Bound::Lower, 2);
    let hit = tt.probe(key, 2).expect("stored");
    assert_eq!(hit.mv, mv(Square::G1, Square::F3));
    assert_eq!(hit.score, -37);
    assert_eq!(hit.depth, 7);
    assert_eq!(hit.bound, Bound::Lower);
    // Same cluster, different upper key bits: a miss, not a false hit.
    assert!(tt.probe(key ^ 0xFFFF_0000_0000_0000, 2).is_none());
    assert!(
        tt.probe(key + 1, 2).is_none()
            || tt.probe(key + 1, 2).unwrap().mv != mv(Square::G1, Square::F3)
    );
}

#[test]
fn mate_scores_are_stored_relative_to_the_node_and_read_relative_to_the_root() {
    let mut tt = TranspositionTable::new(1);
    let key = 0xABCD_EF01_2345_6789;
    // At ply 3 the search found mate in two plies from there: root-relative MATE - 5.
    tt.store(key, Move::NULL, MATE - 5, 3, Bound::Exact, 3);
    // Reached again at ply 7, the same mate is MATE - 9 from the root.
    assert_eq!(tt.probe(key, 7).unwrap().score, MATE - 9);
    assert_eq!(tt.probe(key, 1).unwrap().score, MATE - 3);
    tt.store(key, Move::NULL, -(MATE - 4), 3, Bound::Exact, 2);
    assert_eq!(tt.probe(key, 6).unwrap().score, -(MATE - 8));
}

#[test]
fn a_full_cluster_replaces_the_shallowest_entry_and_keeps_deeper_ones() {
    let mut tt = TranspositionTable::new(1);
    let n = TranspositionTable::ENTRIES_PER_CLUSTER as u64;
    let mask = (tt.clusters() - 1) as u64;
    // Keys that land in the same cluster but differ in the verified bits.
    let keys: Vec<u64> = (0..=n).map(|i| (i << 40) | 5).collect();
    for k in &keys {
        assert_eq!(k & mask, 5 & mask);
    }
    for (i, k) in keys.iter().enumerate().take(n as usize) {
        tt.store(*k, Move::NULL, i as i32, (i as u8 + 1) * 3, Bound::Exact, 0);
    }
    for k in keys.iter().take(n as usize) {
        assert!(tt.probe(*k, 0).is_some(), "all {n} entries fit");
    }
    // One more: the shallowest (depth 3, keys[0]) goes.
    tt.store(keys[n as usize], Move::NULL, 99, 10, Bound::Exact, 0);
    assert!(
        tt.probe(keys[0], 0).is_none(),
        "the shallowest entry was replaced"
    );
    assert!(tt.probe(keys[n as usize], 0).is_some());
    for k in keys.iter().skip(1).take(n as usize - 1) {
        assert!(tt.probe(*k, 0).is_some(), "deeper entries survive");
    }
}

#[test]
fn an_old_generation_is_replaced_before_a_shallow_new_one() {
    let mut tt = TranspositionTable::new(1);
    let n = TranspositionTable::ENTRIES_PER_CLUSTER as u64;
    let keys: Vec<u64> = (0..=n).map(|i| (i << 40) | 9).collect();
    // Fill the cluster in an old search with deep entries.
    for k in keys.iter().take(n as usize - 1) {
        tt.store(*k, Move::NULL, 0, 20, Bound::Exact, 0);
    }
    for _ in 0..4 {
        tt.new_search();
    }
    // A shallow entry from the current search.
    tt.store(keys[n as usize - 1], Move::NULL, 0, 2, Bound::Exact, 0);
    // One more: an old deep entry should go rather than the fresh shallow one.
    tt.store(keys[n as usize], Move::NULL, 0, 6, Bound::Exact, 0);
    assert!(
        tt.probe(keys[n as usize - 1], 0).is_some(),
        "the current search's entry survives"
    );
    assert!(tt.probe(keys[n as usize], 0).is_some());
    let survivors = keys
        .iter()
        .take(n as usize - 1)
        .filter(|k| tt.probe(**k, 0).is_some())
        .count();
    assert_eq!(
        survivors,
        n as usize - 2,
        "exactly one old entry was replaced"
    );
}

#[test]
fn a_shallower_non_exact_store_does_not_overwrite_a_deeper_entry_for_the_same_key() {
    let mut tt = TranspositionTable::new(1);
    let key = 0x1111_2222_3333_4444;
    tt.store(key, mv(Square::E2, Square::E4), 50, 8, Bound::Exact, 0);
    tt.store(key, mv(Square::D2, Square::D4), 10, 2, Bound::Upper, 0);
    let hit = tt.probe(key, 0).unwrap();
    assert_eq!(hit.depth, 8);
    assert_eq!(hit.mv, mv(Square::E2, Square::E4));
    // An exact or deeper store does overwrite.
    tt.store(key, mv(Square::D2, Square::D4), 10, 9, Bound::Lower, 0);
    assert_eq!(tt.probe(key, 0).unwrap().depth, 9);
}

#[test]
fn hashfull_reports_per_mille_of_the_current_generation() {
    let mut tt = TranspositionTable::new(1);
    assert_eq!(tt.hashfull(), 0);
    for i in 0..20_000u64 {
        tt.store(
            i.wrapping_mul(0x9E37_79B9_7F4A_7C15),
            Move::NULL,
            0,
            1,
            Bound::Exact,
            0,
        );
    }
    let full = tt.hashfull();
    assert!(full > 0 && full <= 1000, "{full}");
}

fn search(
    board: &mut Board,
    searcher: &mut Searcher,
    depth: u32,
) -> rusty_chess_search::search::SearchResult {
    searcher.search(
        board,
        &Limits {
            depth: Some(depth),
            ..Limits::default()
        },
        &mut |_| {},
    )
}

#[test]
fn the_table_cuts_nodes_and_keeps_results_deterministic() {
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    searcher.set_hash_mb(16);
    let mut board = Board::from_position(Position::from_fen(fen).unwrap());
    let cold = search(&mut board, &mut searcher, 5);
    let warm = search(&mut board, &mut searcher, 5);
    assert!(
        warm.nodes + warm.qnodes < cold.nodes + cold.qnodes,
        "warm {} vs cold {}",
        warm.nodes + warm.qnodes,
        cold.nodes + cold.qnodes
    );
    assert_eq!(warm.best, cold.best);
    searcher.clear_hash();
    let again = search(&mut board, &mut searcher, 5);
    assert_eq!(
        (again.nodes, again.qnodes, again.best, again.score),
        (cold.nodes, cold.qnodes, cold.best, cold.score),
        "cleared table reproduces the cold search exactly"
    );
    assert!(again.iterations.iter().all(|i| i.tt_hit.is_some()) == cfg!(feature = "stats"));
}

#[test]
fn mates_are_still_exact_with_the_table() {
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    let mut board = Board::from_position(
        Position::from_fen("r2qkb1r/pp2nppp/3p4/2pNN1B1/2BnP3/3P4/PPP2PPP/R2bK2R w KQkq - 1 1")
            .unwrap(),
    );
    let r = search(&mut board, &mut searcher, 5);
    assert_eq!(r.best.to_string(), "d5f6");
    assert_eq!(mate_in(r.score), Some(2));
    let r = search(&mut board, &mut searcher, 6);
    assert_eq!(
        mate_in(r.score),
        Some(2),
        "a warm table must not distort the mate distance"
    );
}

#[test]
fn searching_from_a_transposed_position_reuses_the_table() {
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    let a = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let mut board = Board::from_position(a);
    search(&mut board, &mut searcher, 5);
    let stats_before = searcher.stats().summary();
    let mut other = Board::from_position(
        Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap(),
    );
    let r = search(&mut other, &mut searcher, 5);
    if let (Some(before), Some(after)) = (stats_before, searcher.stats().summary()) {
        assert!(after.tt_hits > before.tt_hits);
    }
    assert!(r.nodes > 0);
}
