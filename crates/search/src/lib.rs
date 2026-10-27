//! Search, transposition table, move picker, evaluation and search telemetry.
//!
//! [`search`] is iterative-deepening alpha-beta with quiescence,
//! [`eval`] is the tapered PeSTO evaluation, [`board`] wraps a position with
//! the incrementally updated evaluation and the key history that search
//! needs, [`stats`] holds the efficiency counters behind the `stats` feature,
//! and [`telemetry`] writes JSON lines.
#![warn(missing_docs)]

pub mod board;
pub mod eval;
pub mod search;
pub mod stats;
pub mod telemetry;
