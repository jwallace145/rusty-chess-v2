//! Zobrist keys: one random 64-bit number per (color, piece kind, square),
//! per castling-rights combination, per en passant file, and one for the
//! side to move. Generated at compile time from a fixed seed, so keys are
//! stable across builds.

/// splitmix64, good enough for 800 well-spread keys.
const fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const SEED: u64 = 0x5EED_C0DE_2026_1026;

/// `PIECE[color][kind][square]`.
pub static PIECE: [[[u64; 64]; 6]; 2] = {
    let mut t = [[[0; 64]; 6]; 2];
    let mut state = SEED;
    let mut c = 0;
    while c < 2 {
        let mut k = 0;
        while k < 6 {
            let mut s = 0;
            while s < 64 {
                t[c][k][s] = splitmix(&mut state);
                s += 1;
            }
            k += 1;
        }
        c += 1;
    }
    t
};

/// `CASTLE[rights]` for each of the 16 combinations.
pub static CASTLE: [u64; 16] = {
    let mut t = [0; 16];
    let mut state = SEED ^ 0xCA57;
    let mut i = 0;
    while i < 16 {
        t[i] = splitmix(&mut state);
        i += 1;
    }
    t[0] = 0; // no rights contributes nothing, so keys stay comparable
    t
};

/// `EP[file]` for the en passant file.
pub static EP: [u64; 8] = {
    let mut t = [0; 8];
    let mut state = SEED ^ 0xE4;
    let mut i = 0;
    while i < 8 {
        t[i] = splitmix(&mut state);
        i += 1;
    }
    t
};

/// Xored in when black is to move.
pub const SIDE: u64 = 0x1D6D_4A2B_8C3F_9E57;
