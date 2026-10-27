//! Developer tools that sit outside the engine binary: the perft suite
//! runner, the performance gate, the PR bench comment, the openings
//! generator, the SPRT report, and (later) data generation for NNUE training.
#![warn(missing_docs)]

pub mod gate;
pub mod history;
pub mod openings;
pub mod perft;
pub mod rating;
pub mod report;
pub mod speed;
pub mod sprt;
