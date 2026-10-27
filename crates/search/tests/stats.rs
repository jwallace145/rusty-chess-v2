//! The `stats` feature: counters that exist (and count) only when enabled.

use rusty_chess_search::stat;
use rusty_chess_search::stats::{SearchStats, Stage, effective_branching_factor};

#[test]
fn stats_struct_is_zero_sized_without_the_feature() {
    if cfg!(feature = "stats") {
        assert!(std::mem::size_of::<SearchStats>() > 0);
    } else {
        assert_eq!(std::mem::size_of::<SearchStats>(), 0);
    }
}

#[test]
fn stat_macro_compiles_in_both_modes_and_counts_only_with_the_feature() {
    let mut stats = SearchStats::default();
    let ply = 3usize;
    stat!(stats, nodes);
    stat!(stats, nodes, 4);
    stat!(stats, qnodes);
    stat!(stats, nodes_by_ply[ply]);
    stat!(stats, fail_high_by_index[0]);
    stat!(stats, fail_high_by_index[0]);
    stat!(stats, fail_high_by_index[2]);
    stat!(stats, cutoffs_by_stage[Stage::Killers as usize]);
    stat!(stats, tt_probes, 10);
    stat!(stats, tt_hits, 5);

    let summary = stats.summary();
    if cfg!(feature = "stats") {
        let s = summary.expect("summary is available with the stats feature");
        assert_eq!(s.nodes, 5);
        assert_eq!(s.qnodes, 1);
        assert_eq!(s.fail_highs, 3);
        assert!((s.fh_first - 2.0 / 3.0).abs() < 1e-9);
        assert!((s.tt_hit - 0.5).abs() < 1e-9);
        assert!((s.qnode_share - 1.0 / 6.0).abs() < 1e-9);
        assert_eq!(s.cutoffs_by_stage[Stage::Killers as usize], 1);
        assert_eq!(s.cutoffs_by_stage[Stage::TtMove as usize], 0);
    } else {
        assert!(summary.is_none());
    }
}

#[test]
fn rates_are_zero_not_nan_when_nothing_was_counted() {
    let stats = SearchStats::default();
    if let Some(s) = stats.summary() {
        assert_eq!(s.fh_first, 0.0);
        assert_eq!(s.tt_hit, 0.0);
        assert_eq!(s.qnode_share, 0.0);
    }
}

#[test]
fn stats_reset_clears_every_counter() {
    let mut stats = SearchStats::default();
    stat!(stats, nodes, 7);
    stat!(stats, null_move_cutoffs);
    stats.reset();
    if let Some(s) = stats.summary() {
        assert_eq!(s.nodes, 0);
        assert_eq!(s.null_move_cutoffs, 0);
    }
}

#[test]
fn effective_branching_factor_is_the_node_ratio_between_iterations() {
    assert_eq!(effective_branching_factor(0, 100), None);
    assert_eq!(effective_branching_factor(100, 0), None);
    let ebf = effective_branching_factor(1_000, 2_500).unwrap();
    assert!((ebf - 2.5).abs() < 1e-9);
}
