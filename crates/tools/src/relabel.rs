//! Re-scoring stored self-play positions with another engine, keeping the
//! positions, their order and their results (`docs/relabel.md`).
//!
//! Each game of a shard is replayed from its `start_fen` with the
//! transposition table cleared, searching every ply at the relabelling
//! budget as datagen did, and playing the game's recorded moves. When the
//! position is the shard's next record, the search's score (white's point of
//! view) becomes its new label. Every record must be matched, in order.

use std::sync::Arc;

use braveheart_core::position::Position;
use braveheart_core::types::Color;
use braveheart_search::board::Board;
use braveheart_search::nnue::Network;
use braveheart_search::search::{Limits, Searcher, is_mate_score};
use serde::Serialize;

use crate::datagen::parse_record;

/// A mate score found where the original search found none is written as
/// this many centipawns, with its sign: the training target saturates long
/// before, and no record is dropped.
pub const MATE_CLAMP: i32 = 3_000;

/// What relabelling one shard changed.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct RelabelStats {
    /// Records relabelled: every record of the shard.
    pub records: u64,
    /// Records whose score changed.
    pub changed: u64,
    /// Sum of |new − old| over all records, centipawns.
    pub sum_abs_change: u64,
    /// The largest |new − old|, centipawns.
    pub max_abs_change: u32,
    /// Mate scores found by the new search, clamped to ±[`MATE_CLAMP`].
    pub mates_clamped: u64,
}

impl RelabelStats {
    /// Add another shard's statistics.
    pub fn add(&mut self, o: &RelabelStats) {
        self.records += o.records;
        self.changed += o.changed;
        self.sum_abs_change += o.sum_abs_change;
        self.max_abs_change = self.max_abs_change.max(o.max_abs_change);
        self.mates_clamped += o.mates_clamped;
    }
}

/// A search score for the side to move as a label from white's point of
/// view, a mate score clamped to ±[`MATE_CLAMP`]. Returns the label and
/// whether it was clamped.
pub fn white_label(score: i32, stm: Color) -> (i32, bool) {
    let white = if stm == Color::White { score } else { -score };
    if is_mate_score(white) {
        (MATE_CLAMP * white.signum(), true)
    } else {
        (white, false)
    }
}

/// Relabel one shard: `positions` is its records file and `games` its games
/// file, both uncompressed. Returns the new records file (the same records,
/// in order, with new scores) and the statistics, or why the shard can't be
/// matched.
pub fn relabel_shard(
    positions: &str,
    games: &str,
    searcher: &mut Searcher,
    net: Option<Arc<Network>>,
    nodes: u64,
) -> Result<(String, RelabelStats), String> {
    let records: Vec<(String, i32, f32)> = positions
        .lines()
        .enumerate()
        .map(|(i, l)| parse_record(l).ok_or_else(|| format!("record {}: unreadable: {l}", i + 1)))
        .collect::<Result<_, _>>()?;
    let limits = Limits {
        nodes: Some(nodes),
        ..Limits::default()
    };
    let mut out = String::with_capacity(positions.len());
    let mut stats = RelabelStats::default();
    let mut next = 0usize;
    for (g, line) in games.lines().enumerate() {
        let game: serde_json::Value =
            serde_json::from_str(line).map_err(|e| format!("game {}: {e}", g + 1))?;
        let start = game["start_fen"]
            .as_str()
            .ok_or_else(|| format!("game {}: no start_fen", g + 1))?;
        let result = game["result"]
            .as_f64()
            .ok_or_else(|| format!("game {}: no result", g + 1))? as f32;
        let moves = game["moves"]
            .as_array()
            .ok_or_else(|| format!("game {}: no moves", g + 1))?;
        let pos = Position::from_fen(start).map_err(|e| format!("game {}: {e:?}", g + 1))?;
        let mut board = Board::from_position(pos);
        board.set_network(net.clone());
        searcher.clear_hash();
        for (ply, m) in moves.iter().enumerate() {
            let fen = board.position().to_fen();
            let stm = board.position().side_to_move();
            let r = searcher.search(&mut board, &limits, &mut |_| {});
            if let Some((rfen, old, rresult)) = records.get(next)
                && *rfen == fen
            {
                if *rresult != result {
                    return Err(format!(
                        "record {}: result {rresult} but its game {} ended {result}",
                        next + 1,
                        g + 1
                    ));
                }
                let (label, clamped) = white_label(r.score, stm);
                let change = label.abs_diff(*old);
                stats.records += 1;
                stats.changed += u64::from(change != 0);
                stats.sum_abs_change += u64::from(change);
                stats.max_abs_change = stats.max_abs_change.max(change);
                stats.mates_clamped += u64::from(clamped);
                out.push_str(&format!("{fen} | {label} | {result:.1}\n"));
                next += 1;
            }
            let text = m
                .as_str()
                .ok_or_else(|| format!("game {} ply {}: not a move", g + 1, ply + 1))?;
            let mv = board.position().parse_uci_move(text).ok_or_else(|| {
                format!(
                    "game {} ply {}: {text} is not legal in {fen}",
                    g + 1,
                    ply + 1
                )
            })?;
            board.make_move(mv);
        }
    }
    if next != records.len() {
        return Err(format!(
            "matched {next} of {} records: record {} ({}) is in no game, in order",
            records.len(),
            next + 1,
            records[next].0
        ));
    }
    Ok((out, stats))
}
