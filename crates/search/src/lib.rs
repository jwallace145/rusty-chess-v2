//! Search, transposition table, move picker, evaluation and search telemetry.
//!
//! Phase 0 of the roadmap establishes the [`stats`] feature and the JSON lines
//! [`telemetry`] writer; Phase 2 adds the search itself.
#![warn(missing_docs)]

pub mod stats;
pub mod telemetry;
