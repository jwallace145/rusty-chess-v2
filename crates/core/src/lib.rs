//! Board representation, bitboards, magic move generation, make/unmake and
//! Zobrist hashing.
//!
//! This crate has no dependencies and no heap allocation on any path that
//! search will call. Phase 1 of the roadmap fills it in; until then it only
//! establishes the crate boundary.
#![warn(missing_docs)]
