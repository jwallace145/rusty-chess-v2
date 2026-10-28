//! Search statistics counters.
//!
//! [`SearchStats`] holds the counters that measure search *efficiency*, as
//! opposed to speed: fail-high-first rate, transposition table hit rate,
//! cutoffs by move-picker stage, and pruning events. They are only compiled in
//! with the `stats` Cargo feature. Without it [`SearchStats`] is a zero-sized
//! type and every [`stat!`](crate::stat) invocation expands to nothing, so the
//! release build pays nothing for them.
//!
//! Every increment in the search goes through the macro, never through a
//! direct field access, so that the feature can be switched off.

use serde::{Deserialize, Serialize};

/// The deepest ply the per-ply counters track.
pub const MAX_PLY: usize = 128;

/// Fail-highs are bucketed by the index of the move that caused them, up to
/// this many; later moves share the last bucket.
pub const MAX_MOVE_INDEX: usize = 64;

/// Move-picker stages, in the order they are searched. Cutoffs are counted
/// per stage so that a weak stage shows up directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum Stage {
    /// The move from the transposition table.
    TtMove = 0,
    /// Captures with non-negative static exchange evaluation, MVV-LVA ordered.
    GoodCaptures = 1,
    /// Killer moves for this ply.
    Killers = 2,
    /// Quiet moves ordered by butterfly history.
    HistoryQuiets = 3,
    /// Captures that lose material by static exchange evaluation.
    BadCaptures = 4,
}

/// Number of [`Stage`]s.
pub const STAGES: usize = 5;

/// Increment a counter in a [`SearchStats`], or nothing without the `stats`
/// feature.
///
/// ```ignore
/// stat!(stats, nodes);                    // += 1
/// stat!(stats, tt_probes, 3);             // += 3
/// stat!(stats, nodes_by_ply[ply]);        // indexed, += 1
/// stat!(stats, fail_high_by_index[i], 1); // indexed, += n
/// ```
///
/// Without the feature the stats value is still borrowed mutably and the
/// index expression is still named, so that callers declare the same `mut`
/// bindings in both modes and nothing becomes an unused variable; neither is
/// evaluated for effect.
#[macro_export]
macro_rules! stat {
    ($stats:expr, $field:ident) => {
        $crate::stat!($stats, $field, 1)
    };
    ($stats:expr, $field:ident [ $idx:expr ]) => {
        $crate::stat!($stats, $field[$idx], 1)
    };
    ($stats:expr, $field:ident, $n:expr) => {{
        #[cfg(feature = "stats")]
        {
            $stats.$field += ($n) as u64;
        }
        #[cfg(not(feature = "stats"))]
        {
            let _ = &mut $stats;
        }
    }};
    ($stats:expr, $field:ident [ $idx:expr ], $n:expr) => {{
        #[cfg(feature = "stats")]
        {
            let idx: usize = $idx;
            let slot = &mut $stats.$field;
            let last = slot.len() - 1;
            slot[if idx < last { idx } else { last }] += ($n) as u64;
        }
        #[cfg(not(feature = "stats"))]
        {
            let _ = (&mut $stats, &$idx);
        }
    }};
}

/// Search statistics counters. See the [module docs](self).
#[cfg(feature = "stats")]
#[derive(Clone, Debug)]
pub struct SearchStats {
    /// Nodes visited in the main search, including the root.
    pub nodes: u64,
    /// Nodes visited in quiescence search.
    pub qnodes: u64,
    /// Main-search nodes per ply from the root.
    pub nodes_by_ply: [u64; MAX_PLY],
    /// Quiescence nodes per ply from the root.
    pub qnodes_by_ply: [u64; MAX_PLY],
    /// Transposition table probes.
    pub tt_probes: u64,
    /// Probes that found an entry for this position.
    pub tt_hits: u64,
    /// Hits whose bound and depth allowed an immediate cutoff.
    pub tt_cutoffs: u64,
    /// Probes that found an entry for a different position (key mismatch).
    pub tt_collisions: u64,
    /// Fail-highs bucketed by the index of the move that caused them.
    /// Index 0 is the fail-high-first count.
    pub fail_high_by_index: [u64; MAX_MOVE_INDEX],
    /// Fail-highs bucketed by move-picker [`Stage`].
    pub cutoffs_by_stage: [u64; STAGES],
    /// Nodes returned by reverse futility pruning.
    pub rfp_cutoffs: u64,
    /// Null-move searches that failed high.
    pub null_move_cutoffs: u64,
    /// Moves searched with a late-move reduction.
    pub lmr_reductions: u64,
    /// Reduced moves that had to be re-searched at full depth.
    pub lmr_researches: u64,
    /// Moves skipped by futility pruning.
    pub futility_skips: u64,
    /// Quiet moves skipped by late move pruning.
    pub lmp_skips: u64,
    /// Nodes ended by mate-distance pruning.
    pub mdp_cutoffs: u64,
    /// Quiescence captures skipped for losing material by static exchange.
    pub qsee_skips: u64,
    /// Transposition table probes from quiescence.
    pub qtt_probes: u64,
    /// Quiescence nodes ended by a transposition table hit.
    pub qtt_cutoffs: u64,
    /// Quiet moves late move pruning would have skipped but kept because
    /// they give check.
    pub lmp_checks_kept: u64,
    /// Aspiration windows that failed and were re-searched wider.
    pub aspiration_researches: u64,
}

#[cfg(feature = "stats")]
impl Default for SearchStats {
    fn default() -> Self {
        Self {
            nodes: 0,
            qnodes: 0,
            nodes_by_ply: [0; MAX_PLY],
            qnodes_by_ply: [0; MAX_PLY],
            tt_probes: 0,
            tt_hits: 0,
            tt_cutoffs: 0,
            tt_collisions: 0,
            fail_high_by_index: [0; MAX_MOVE_INDEX],
            cutoffs_by_stage: [0; STAGES],
            rfp_cutoffs: 0,
            null_move_cutoffs: 0,
            lmr_reductions: 0,
            lmr_researches: 0,
            futility_skips: 0,
            lmp_skips: 0,
            mdp_cutoffs: 0,
            qsee_skips: 0,
            qtt_probes: 0,
            qtt_cutoffs: 0,
            lmp_checks_kept: 0,
            aspiration_researches: 0,
        }
    }
}

#[cfg(feature = "stats")]
impl SearchStats {
    /// Fresh counters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Zero every counter.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The derived rates and totals for telemetry.
    pub fn summary(&self) -> Option<StatsSummary> {
        let fail_highs: u64 = self.fail_high_by_index.iter().sum();
        Some(StatsSummary {
            nodes: self.nodes,
            qnodes: self.qnodes,
            qnode_share: ratio(self.qnodes, self.nodes + self.qnodes),
            tt_probes: self.tt_probes,
            tt_hits: self.tt_hits,
            tt_cutoffs: self.tt_cutoffs,
            tt_collisions: self.tt_collisions,
            tt_hit: ratio(self.tt_hits, self.tt_probes),
            fail_highs,
            fh_first: ratio(self.fail_high_by_index[0], fail_highs),
            cutoffs_by_stage: self.cutoffs_by_stage,
            rfp_cutoffs: self.rfp_cutoffs,
            null_move_cutoffs: self.null_move_cutoffs,
            lmr_reductions: self.lmr_reductions,
            lmr_researches: self.lmr_researches,
            futility_skips: self.futility_skips,
            lmp_skips: self.lmp_skips,
            mdp_cutoffs: self.mdp_cutoffs,
            qsee_skips: self.qsee_skips,
            qtt_probes: self.qtt_probes,
            qtt_cutoffs: self.qtt_cutoffs,
            lmp_checks_kept: self.lmp_checks_kept,
            aspiration_researches: self.aspiration_researches,
        })
    }
}

/// Search statistics counters. Without the `stats` feature this is a
/// zero-sized placeholder so that search code can hold one unconditionally.
#[cfg(not(feature = "stats"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct SearchStats;

#[cfg(not(feature = "stats"))]
impl SearchStats {
    /// Fresh counters (nothing, without the `stats` feature).
    pub fn new() -> Self {
        SearchStats
    }

    /// Zero every counter (a no-op without the `stats` feature).
    pub fn reset(&mut self) {}

    /// The derived rates and totals; `None` without the `stats` feature.
    pub fn summary(&self) -> Option<StatsSummary> {
        None
    }
}

/// Derived totals and rates from a [`SearchStats`], as written to telemetry.
/// Rates are in `0.0..=1.0` and are `0.0` rather than NaN when nothing was
/// counted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StatsSummary {
    /// Main-search nodes.
    pub nodes: u64,
    /// Quiescence nodes.
    pub qnodes: u64,
    /// `qnodes / (nodes + qnodes)`.
    pub qnode_share: f64,
    /// Transposition table probes.
    pub tt_probes: u64,
    /// Probes that hit.
    pub tt_hits: u64,
    /// Hits that cut off.
    pub tt_cutoffs: u64,
    /// Probes that found another position's entry.
    pub tt_collisions: u64,
    /// `tt_hits / tt_probes`.
    pub tt_hit: f64,
    /// Fail-highs in total.
    pub fail_highs: u64,
    /// Share of fail-highs caused by the first move searched. The Phase 3
    /// gate is `>= 0.90`.
    pub fh_first: f64,
    /// Fail-highs by [`Stage`], indexed by `Stage as usize`.
    pub cutoffs_by_stage: [u64; STAGES],
    /// Reverse futility cutoffs. Absent in records written before it existed.
    #[serde(default)]
    pub rfp_cutoffs: u64,
    /// Null-move cutoffs.
    pub null_move_cutoffs: u64,
    /// Late-move reductions applied.
    pub lmr_reductions: u64,
    /// Reduced moves re-searched at full depth.
    pub lmr_researches: u64,
    /// Futility-pruned moves.
    pub futility_skips: u64,
    /// Quiet moves skipped by late move pruning. Absent in records written before it existed.
    #[serde(default)]
    pub lmp_skips: u64,
    /// Nodes ended by mate-distance pruning. Absent in records written before it existed.
    #[serde(default)]
    pub mdp_cutoffs: u64,
    /// Quiescence captures skipped by static exchange. Absent in records written before it existed.
    #[serde(default)]
    pub qsee_skips: u64,
    /// Transposition table probes from quiescence. Absent in records written before it existed.
    #[serde(default)]
    pub qtt_probes: u64,
    /// Quiescence nodes ended by a table hit. Absent in records written before it existed.
    #[serde(default)]
    pub qtt_cutoffs: u64,
    /// Checking moves kept by late move pruning. Absent in records written before it existed.
    #[serde(default)]
    pub lmp_checks_kept: u64,
    /// Aspiration windows re-searched wider. Absent in records written before it existed.
    #[serde(default)]
    pub aspiration_researches: u64,
}

/// Effective branching factor between two consecutive iterations of
/// iterative deepening: `nodes / prev_nodes`. `None` if either is zero.
pub fn effective_branching_factor(prev_nodes: u64, nodes: u64) -> Option<f64> {
    if prev_nodes == 0 || nodes == 0 {
        None
    } else {
        Some(nodes as f64 / prev_nodes as f64)
    }
}

#[cfg(feature = "stats")]
fn ratio(num: u64, den: u64) -> f64 {
    if den == 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}
