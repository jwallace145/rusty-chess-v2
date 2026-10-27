//! Killer moves: per ply, the two most recent quiet moves that caused a
//! beta cutoff. A move that refutes one line at a ply often refutes its
//! siblings too, so the picker tries killers right after the good captures.

use rusty_chess_core::r#move::Move;

use crate::stats::MAX_PLY;

/// Two killer slots for every ply.
#[derive(Clone)]
pub struct Killers {
    slots: [[Move; 2]; MAX_PLY],
}

impl Default for Killers {
    fn default() -> Self {
        Self::new()
    }
}

impl Killers {
    /// Empty slots.
    pub fn new() -> Killers {
        Killers {
            slots: [[Move::NULL; 2]; MAX_PLY],
        }
    }

    /// Forget every killer (a new search).
    pub fn clear(&mut self) {
        self.slots = [[Move::NULL; 2]; MAX_PLY];
    }

    /// The killers at `ply`, newest first.
    #[inline]
    pub fn get(&self, ply: usize) -> [Move; 2] {
        self.slots[ply]
    }

    /// Record `m` as the newest killer at `ply`; the older slot falls out.
    #[inline]
    pub fn store(&mut self, ply: usize, m: Move) {
        let slot = &mut self.slots[ply];
        if slot[0] != m {
            slot[1] = slot[0];
            slot[0] = m;
        }
    }
}
