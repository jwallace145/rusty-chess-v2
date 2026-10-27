//! Board representation, bitboards, magic move generation, make/unmake and
//! Zobrist hashing.
//!
//! This crate has no dependencies and no heap allocation on any path that
//! search will call. Lookup tables are either `const` or built once on first
//! use.
#![warn(missing_docs)]

pub mod attacks;
pub mod bitboard;
pub mod types;
