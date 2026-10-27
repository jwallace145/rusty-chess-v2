//! The transposition table.
//!
//! Entries are 10 bytes packed into 32-byte clusters of three, two clusters
//! per cache line. The upper 32 bits of the Zobrist key verify a hit; the
//! low bits pick the cluster. Replacement prefers an empty slot or the same
//! key, then the entry worth least: shallow and old. Mate scores are stored
//! relative to the node and converted back to root-relative on probe, so a
//! mate found at one ply reads correctly at another.
//!
//! Rules from the v1 post-mortem that this table enforces by construction:
//! the bound stored is derived by the caller from the score against the
//! original window, and a shallower non-exact store never overwrites a
//! deeper entry for the same key.

use rusty_chess_core::r#move::Move;

use crate::search::MATE_BOUND;

/// What a stored score means relative to the window it was searched with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Bound {
    /// The score is exact: it fell inside the window.
    Exact = 1,
    /// A lower bound: the search failed high, the true score is at least this.
    Lower = 2,
    /// An upper bound: the search failed low, the true score is at most this.
    Upper = 3,
}

/// One stored position.
#[derive(Clone, Copy, Default)]
#[repr(C)]
struct Entry {
    key: u32,
    mv: u16,
    score: i16,
    depth: u8,
    gen_bound: u8, // generation in the high 6 bits, bound in the low 2
}

impl Entry {
    #[inline]
    fn bound(&self) -> Option<Bound> {
        match self.gen_bound & 3 {
            1 => Some(Bound::Exact),
            2 => Some(Bound::Lower),
            3 => Some(Bound::Upper),
            _ => None,
        }
    }

    #[inline]
    fn generation(&self) -> u8 {
        self.gen_bound >> 2
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.gen_bound & 3 == 0
    }
}

const ENTRIES: usize = 3;

#[derive(Clone, Copy, Default)]
#[repr(C, align(32))]
struct Cluster {
    entries: [Entry; ENTRIES],
    _pad: [u8; 2],
}

/// What a probe found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    /// The best or cutoff move stored, `Move::NULL` if none.
    pub mv: Move,
    /// The score, root-relative.
    pub score: i32,
    /// The depth it was searched to.
    pub depth: u8,
    /// What the score means.
    pub bound: Bound,
}

/// The table.
pub struct TranspositionTable {
    clusters: Vec<Cluster>,
    mask: usize,
    generation: u8,
}

impl TranspositionTable {
    /// Bytes per entry.
    pub const ENTRY_BYTES: usize = std::mem::size_of::<Entry>();
    /// Bytes per cluster.
    pub const CLUSTER_BYTES: usize = std::mem::size_of::<Cluster>();
    /// Entries per cluster.
    pub const ENTRIES_PER_CLUSTER: usize = ENTRIES;

    /// A table of about `mb` megabytes (rounded down to a power of two of
    /// clusters, at least one).
    pub fn new(mb: usize) -> TranspositionTable {
        let mut tt = TranspositionTable {
            clusters: Vec::new(),
            mask: 0,
            generation: 0,
        };
        tt.resize(mb);
        tt
    }

    /// Reallocate to about `mb` megabytes and clear.
    pub fn resize(&mut self, mb: usize) {
        let bytes = mb.max(1) * 1024 * 1024;
        let wanted = bytes / Self::CLUSTER_BYTES;
        // The largest power of two that fits.
        let count = if wanted.is_power_of_two() {
            wanted
        } else {
            wanted.next_power_of_two() / 2
        }
        .max(1);
        self.clusters = vec![Cluster::default(); count];
        self.mask = count - 1;
        self.generation = 0;
    }

    /// Forget everything.
    pub fn clear(&mut self) {
        self.clusters.fill(Cluster::default());
        self.generation = 0;
    }

    /// How many clusters the table has.
    pub fn clusters(&self) -> usize {
        self.clusters.len()
    }

    /// Start a new search: entries stored from now on outrank older ones.
    pub fn new_search(&mut self) {
        self.generation = (self.generation + 1) & 63;
    }

    /// Per mille of the first thousand entries that belong to the current
    /// search, the UCI `hashfull`.
    pub fn hashfull(&self) -> u32 {
        let mut used = 0;
        let mut seen = 0;
        for cluster in self.clusters.iter().take(1000 / ENTRIES + 1) {
            for e in &cluster.entries {
                if seen == 1000 {
                    break;
                }
                seen += 1;
                if !e.is_empty() && e.generation() == self.generation {
                    used += 1;
                }
            }
        }
        used * 1000 / seen.max(1)
    }

    /// Look `key` up; the score comes back relative to the root for a node
    /// at `ply`.
    #[inline]
    pub fn probe(&self, key: u64, ply: usize) -> Option<Hit> {
        let cluster = &self.clusters[(key as usize) & self.mask];
        let verify = (key >> 32) as u32;
        cluster
            .entries
            .iter()
            .find(|e| !e.is_empty() && e.key == verify)
            .map(|e| Hit {
                mv: Move::from_bits(e.mv),
                score: from_tt_score(e.score as i32, ply),
                depth: e.depth,
                bound: e.bound().expect("non-empty entry has a bound"),
            })
    }

    /// Store a result for `key` found at `ply`. A shallower, non-exact
    /// result does not overwrite a deeper entry for the same key.
    pub fn store(&mut self, key: u64, mv: Move, score: i32, depth: u8, bound: Bound, ply: usize) {
        let generation = self.generation;
        let cluster = &mut self.clusters[(key as usize) & self.mask];
        let verify = (key >> 32) as u32;

        let slot = match cluster
            .entries
            .iter()
            .position(|e| e.is_empty() || e.key == verify)
        {
            Some(i) => i,
            None => {
                // Replace the entry worth least: shallow, and from an old search.
                let worth = |e: &Entry| {
                    e.depth as i32 - 8 * ((generation.wrapping_sub(e.generation())) & 63) as i32
                };
                let mut best = 0;
                for i in 1..ENTRIES {
                    if worth(&cluster.entries[i]) < worth(&cluster.entries[best]) {
                        best = i;
                    }
                }
                best
            }
        };
        let e = &mut cluster.entries[slot];
        let same = !e.is_empty() && e.key == verify;
        if same && bound != Bound::Exact && depth < e.depth.saturating_sub(3) {
            return; // keep the deeper entry; a shallow bound adds nothing
        }
        let mv = if mv == Move::NULL && same {
            e.mv
        } else {
            mv.bits()
        };
        *e = Entry {
            key: verify,
            mv,
            score: to_tt_score(score, ply) as i16,
            depth,
            gen_bound: (generation << 2) | bound as u8,
        };
    }
}

/// Root-relative mate scores become node-relative for storage.
#[inline]
fn to_tt_score(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score + ply as i32
    } else if score <= -MATE_BOUND {
        score - ply as i32
    } else {
        score
    }
}

/// Node-relative mate scores become root-relative on probe.
#[inline]
fn from_tt_score(score: i32, ply: usize) -> i32 {
    if score >= MATE_BOUND {
        score - ply as i32
    } else if score <= -MATE_BOUND {
        score + ply as i32
    } else {
        score
    }
}
