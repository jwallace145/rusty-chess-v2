//! Attack tables.
//!
//! Leapers (knight, king, pawn) and the between/line tables are `const`.
//! Sliders use fancy magic bitboards: a per-square magic number maps the
//! relevant occupancy to an index into one shared attack table, which is
//! built once on first use from the magic numbers in [`magics`].

use std::sync::LazyLock;

use crate::bitboard::Bitboard;
use crate::types::{Color, Square};

mod magics;

const fn leaper(sq: usize, deltas: &[(i8, i8)]) -> u64 {
    let (file, rank) = ((sq & 7) as i8, (sq >> 3) as i8);
    let mut out = 0u64;
    let mut i = 0;
    while i < deltas.len() {
        let (f, r) = (file + deltas[i].0, rank + deltas[i].1);
        if f >= 0 && f < 8 && r >= 0 && r < 8 {
            out |= 1u64 << (r * 8 + f);
        }
        i += 1;
    }
    out
}

/// Ray attacks from `sq` in `deltas` directions, stopping at (and including)
/// the first occupied square.
const fn slider(sq: usize, occ: u64, deltas: &[(i8, i8)]) -> u64 {
    let mut out = 0u64;
    let mut i = 0;
    while i < deltas.len() {
        let (df, dr) = deltas[i];
        let (mut f, mut r) = ((sq & 7) as i8 + df, (sq >> 3) as i8 + dr);
        while f >= 0 && f < 8 && r >= 0 && r < 8 {
            let bit = 1u64 << (r * 8 + f);
            out |= bit;
            if occ & bit != 0 {
                break;
            }
            f += df;
            r += dr;
        }
        i += 1;
    }
    out
}

const KNIGHT_DELTAS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];
const KING_DELTAS: [(i8, i8); 8] = [
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
    (-1, -1),
    (-1, 0),
    (-1, 1),
];
const ROOK_DELTAS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP_DELTAS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

const KNIGHT: [u64; 64] = {
    let mut t = [0; 64];
    let mut sq = 0;
    while sq < 64 {
        t[sq] = leaper(sq, &KNIGHT_DELTAS);
        sq += 1;
    }
    t
};

const KING: [u64; 64] = {
    let mut t = [0; 64];
    let mut sq = 0;
    while sq < 64 {
        t[sq] = leaper(sq, &KING_DELTAS);
        sq += 1;
    }
    t
};

const PAWN: [[u64; 64]; 2] = {
    let mut t = [[0; 64]; 2];
    let mut sq = 0;
    while sq < 64 {
        t[0][sq] = leaper(sq, &[(-1, 1), (1, 1)]);
        t[1][sq] = leaper(sq, &[(-1, -1), (1, -1)]);
        sq += 1;
    }
    t
};

/// `BETWEEN[a][b]`: the squares strictly between `a` and `b` if they share
/// a rank, file or diagonal; otherwise empty.
static BETWEEN: [[u64; 64]; 64] = {
    let mut t = [[0; 64]; 64];
    let mut a = 0;
    while a < 64 {
        let mut b = 0;
        while b < 64 {
            let bit_b = 1u64 << b;
            if a != b {
                if slider(a, 0, &ROOK_DELTAS) & bit_b != 0 {
                    t[a][b] = slider(a, bit_b, &ROOK_DELTAS) & slider(b, 1u64 << a, &ROOK_DELTAS);
                } else if slider(a, 0, &BISHOP_DELTAS) & bit_b != 0 {
                    t[a][b] =
                        slider(a, bit_b, &BISHOP_DELTAS) & slider(b, 1u64 << a, &BISHOP_DELTAS);
                }
            }
            b += 1;
        }
        a += 1;
    }
    t
};

/// `LINE[a][b]`: the whole rank, file or diagonal through `a` and `b`
/// (both included), or empty if they do not share one.
static LINE: [[u64; 64]; 64] = {
    let mut t = [[0; 64]; 64];
    let mut a = 0;
    while a < 64 {
        let mut b = 0;
        while b < 64 {
            let bit_b = 1u64 << b;
            if a != b {
                if slider(a, 0, &ROOK_DELTAS) & bit_b != 0 {
                    t[a][b] = (slider(a, 0, &ROOK_DELTAS) & slider(b, 0, &ROOK_DELTAS))
                        | (1u64 << a)
                        | bit_b;
                } else if slider(a, 0, &BISHOP_DELTAS) & bit_b != 0 {
                    t[a][b] = (slider(a, 0, &BISHOP_DELTAS) & slider(b, 0, &BISHOP_DELTAS))
                        | (1u64 << a)
                        | bit_b;
                }
            }
            b += 1;
        }
        a += 1;
    }
    t
};

/// Squares a knight on `sq` attacks.
#[inline]
pub fn knight_attacks(sq: Square) -> Bitboard {
    Bitboard(KNIGHT[sq.index()])
}

/// Squares a king on `sq` attacks.
#[inline]
pub fn king_attacks(sq: Square) -> Bitboard {
    Bitboard(KING[sq.index()])
}

/// Squares a pawn of `color` on `sq` attacks.
#[inline]
pub fn pawn_attacks(color: Color, sq: Square) -> Bitboard {
    Bitboard(PAWN[color.index()][sq.index()])
}

/// The squares strictly between `a` and `b` along a rank, file or diagonal;
/// empty if they are not aligned or adjacent.
#[inline]
pub fn between(a: Square, b: Square) -> Bitboard {
    Bitboard(BETWEEN[a.index()][b.index()])
}

/// The full rank, file or diagonal through `a` and `b`; empty if they are
/// not aligned.
#[inline]
pub fn line(a: Square, b: Square) -> Bitboard {
    Bitboard(LINE[a.index()][b.index()])
}

/// A square's magic: the relevant-occupancy mask, the multiplier, the shift
/// and where its attacks start in the shared table.
#[derive(Clone, Copy)]
struct Magic {
    mask: u64,
    magic: u64,
    shift: u8,
    offset: u32,
}

impl Magic {
    #[inline]
    const fn index(&self, occ: u64) -> usize {
        self.offset as usize + ((occ & self.mask).wrapping_mul(self.magic) >> self.shift) as usize
    }
}

/// The relevant occupancy mask: the slider's rays without the board edges
/// (an edge square's occupancy never changes the attack set).
const fn relevant_mask(sq: usize, deltas: &[(i8, i8)]) -> u64 {
    let mut out = 0u64;
    let mut i = 0;
    while i < deltas.len() {
        let (df, dr) = deltas[i];
        let (mut f, mut r) = ((sq & 7) as i8 + df, (sq >> 3) as i8 + dr);
        while f + df >= 0 && f + df < 8 && r + dr >= 0 && r + dr < 8 {
            out |= 1u64 << (r * 8 + f);
            f += df;
            r += dr;
        }
        i += 1;
    }
    out
}

const fn build_magics(numbers: &[u64; 64], deltas: &[(i8, i8)]) -> ([Magic; 64], usize) {
    let mut out = [Magic {
        mask: 0,
        magic: 0,
        shift: 0,
        offset: 0,
    }; 64];
    let mut offset = 0usize;
    let mut sq = 0;
    while sq < 64 {
        let mask = relevant_mask(sq, deltas);
        let bits = mask.count_ones();
        out[sq] = Magic {
            mask,
            magic: numbers[sq],
            shift: (64 - bits) as u8,
            offset: offset as u32,
        };
        offset += 1 << bits;
        sq += 1;
    }
    (out, offset)
}

const ROOK_MAGICS: ([Magic; 64], usize) = build_magics(&magics::ROOK, &ROOK_DELTAS);
const BISHOP_MAGICS: ([Magic; 64], usize) = build_magics(&magics::BISHOP, &BISHOP_DELTAS);

/// Fill one slider's table by enumerating every subset of each square's mask.
fn build_table(magics: &[Magic; 64], size: usize, deltas: &[(i8, i8)]) -> Box<[u64]> {
    let mut table = vec![0u64; size].into_boxed_slice();
    for (sq, m) in magics.iter().enumerate() {
        let mut occ = 0u64;
        loop {
            let idx = m.index(occ);
            let attacks = slider(sq, occ, deltas);
            debug_assert!(
                table[idx] == 0 || table[idx] == attacks,
                "magic collision on square {sq}"
            );
            table[idx] = attacks;
            occ = occ.wrapping_sub(m.mask) & m.mask; // next subset of the mask
            if occ == 0 {
                break;
            }
        }
    }
    table
}

static ROOK_TABLE: LazyLock<Box<[u64]>> =
    LazyLock::new(|| build_table(&ROOK_MAGICS.0, ROOK_MAGICS.1, &ROOK_DELTAS));
static BISHOP_TABLE: LazyLock<Box<[u64]>> =
    LazyLock::new(|| build_table(&BISHOP_MAGICS.0, BISHOP_MAGICS.1, &BISHOP_DELTAS));

/// Squares a rook on `sq` attacks given the occupancy `occ` (blockers
/// included; `sq` itself may or may not be in `occ`).
#[inline]
pub fn rook_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    let m = &ROOK_MAGICS.0[sq.index()];
    Bitboard(ROOK_TABLE[m.index(occ.0)])
}

/// Squares a bishop on `sq` attacks given the occupancy `occ`.
#[inline]
pub fn bishop_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    let m = &BISHOP_MAGICS.0[sq.index()];
    Bitboard(BISHOP_TABLE[m.index(occ.0)])
}

/// Squares a queen on `sq` attacks given the occupancy `occ`.
#[inline]
pub fn queen_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}

/// Force the slider tables to be built now rather than on the first probe.
pub fn init() {
    LazyLock::force(&ROOK_TABLE);
    LazyLock::force(&BISHOP_TABLE);
}

#[cfg(test)]
mod find {
    //! The magic finder that produced `magics.rs`. Run it with
    //! `cargo test -p rusty-chess-core --release -- --ignored find_magics --nocapture`
    //! and paste the output; it is deterministic for a given seed.
    use super::*;

    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn sparse(&mut self) -> u64 {
            self.next() & self.next() & self.next()
        }
    }

    fn find(sq: usize, deltas: &[(i8, i8)], rng: &mut Rng) -> u64 {
        let mask = relevant_mask(sq, deltas);
        let bits = mask.count_ones();
        let shift = 64 - bits;
        let mut subsets = Vec::new();
        let mut occ = 0u64;
        loop {
            subsets.push((occ, slider(sq, occ, deltas)));
            occ = occ.wrapping_sub(mask) & mask;
            if occ == 0 {
                break;
            }
        }
        let mut table = vec![0u64; 1 << bits];
        let mut stamp = vec![0u32; 1 << bits];
        let mut attempt = 0u32;
        loop {
            let magic = rng.sparse();
            // A magic that maps the mask to few high bits is hopeless; skip fast.
            if ((mask.wrapping_mul(magic)) >> 56).count_ones() < 6 {
                continue;
            }
            attempt += 1;
            let mut ok = true;
            for &(occ, attacks) in &subsets {
                let idx = (occ.wrapping_mul(magic) >> shift) as usize;
                if stamp[idx] != attempt {
                    stamp[idx] = attempt;
                    table[idx] = attacks;
                } else if table[idx] != attacks {
                    ok = false;
                    break;
                }
            }
            if ok {
                return magic;
            }
        }
    }

    #[test]
    #[ignore = "generator for magics.rs; run on demand"]
    fn find_magics() {
        let mut rng = Rng(0x2545_F491_4F6C_DD1D);
        for (name, deltas) in [("ROOK", &ROOK_DELTAS[..]), ("BISHOP", &BISHOP_DELTAS[..])] {
            println!("pub(super) const {name}: [u64; 64] = [");
            for sq in 0..64 {
                let m = find(sq, deltas, &mut rng);
                print!("    {m:#018x},");
                if sq % 4 == 3 {
                    println!();
                }
            }
            println!("];");
        }
    }
}
