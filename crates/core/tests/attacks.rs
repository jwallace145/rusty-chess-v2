//! Attack tables: leapers, sliders through magic bitboards, and the lines
//! between squares. Everything is checked against a slow reference.

use rusty_chess_core::attacks::{
    between, bishop_attacks, king_attacks, knight_attacks, line, pawn_attacks, queen_attacks,
    rook_attacks,
};
use rusty_chess_core::bitboard::Bitboard;
use rusty_chess_core::types::{Color, Square};

fn bb(squares: &[Square]) -> Bitboard {
    squares.iter().fold(Bitboard::EMPTY, |acc, &s| acc.with(s))
}

/// Walk rays square by square: the reference the magic tables must match.
fn slow_slider(sq: Square, occ: Bitboard, deltas: &[(i8, i8)]) -> Bitboard {
    let mut out = Bitboard::EMPTY;
    for &(df, dr) in deltas {
        let mut cur = sq;
        while let Some(next) = cur.offset(df, dr) {
            out = out.with(next);
            if occ.contains(next) {
                break;
            }
            cur = next;
        }
    }
    out
}

const ROOK: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const BISHOP: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

#[test]
fn knight_and_king_leapers() {
    assert_eq!(knight_attacks(Square::A1), bb(&[Square::B3, Square::C2]));
    assert_eq!(knight_attacks(Square::E4).count(), 8);
    assert_eq!(
        king_attacks(Square::A1),
        bb(&[Square::A2, Square::B1, Square::B2])
    );
    assert_eq!(king_attacks(Square::E4).count(), 8);
    assert!(!king_attacks(Square::H8).contains(Square::A8));
}

#[test]
fn pawn_attacks_by_color_and_edge() {
    assert_eq!(
        pawn_attacks(Color::White, Square::E4),
        bb(&[Square::D5, Square::F5])
    );
    assert_eq!(
        pawn_attacks(Color::Black, Square::E4),
        bb(&[Square::D3, Square::F3])
    );
    assert_eq!(pawn_attacks(Color::White, Square::A2), bb(&[Square::B3]));
    assert_eq!(pawn_attacks(Color::White, Square::H7), bb(&[Square::G8]));
    assert_eq!(pawn_attacks(Color::Black, Square::H2), bb(&[Square::G1]));
}

#[test]
fn sliders_on_an_empty_board() {
    assert_eq!(rook_attacks(Square::A1, Bitboard::EMPTY).count(), 14);
    assert_eq!(bishop_attacks(Square::A1, Bitboard::EMPTY).count(), 7);
    assert_eq!(bishop_attacks(Square::E4, Bitboard::EMPTY).count(), 13);
    assert_eq!(queen_attacks(Square::D4, Bitboard::EMPTY).count(), 27);
}

#[test]
fn sliders_stop_at_blockers_and_include_them() {
    let occ = bb(&[Square::E6, Square::B4, Square::E2]);
    let r = rook_attacks(Square::E4, occ);
    assert!(r.contains(Square::E6) && !r.contains(Square::E7));
    assert!(r.contains(Square::B4) && !r.contains(Square::A4));
    assert!(r.contains(Square::E2) && !r.contains(Square::E1));
    assert!(r.contains(Square::H4));
    assert_eq!(r, slow_slider(Square::E4, occ, &ROOK));
}

#[test]
fn magic_tables_match_the_slow_reference_on_random_occupancies() {
    // A fixed-seed xorshift so the test is deterministic.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in 0..64u8 {
        let sq = Square::from_index(i);
        for _ in 0..200 {
            // Sparse-ish occupancy so rays are sometimes long.
            let occ = Bitboard(next() & next());
            assert_eq!(
                rook_attacks(sq, occ),
                slow_slider(sq, occ, &ROOK),
                "rook {sq} {occ}"
            );
            assert_eq!(
                bishop_attacks(sq, occ),
                slow_slider(sq, occ, &BISHOP),
                "bishop {sq} {occ}"
            );
        }
        // The slider's own square in the occupancy must not matter.
        let occ = Bitboard(next()).with(sq);
        assert_eq!(rook_attacks(sq, occ), rook_attacks(sq, occ.without(sq)));
    }
}

#[test]
fn between_and_line() {
    assert_eq!(
        between(Square::A1, Square::A4),
        bb(&[Square::A2, Square::A3])
    );
    assert_eq!(
        between(Square::A1, Square::D4),
        bb(&[Square::B2, Square::C3])
    );
    assert_eq!(between(Square::A1, Square::B3), Bitboard::EMPTY);
    assert_eq!(between(Square::A1, Square::A2), Bitboard::EMPTY);
    assert_eq!(between(Square::E4, Square::E4), Bitboard::EMPTY);
    assert_eq!(line(Square::A1, Square::B3), Bitboard::EMPTY);
    assert_eq!(line(Square::A1, Square::H8).count(), 8);
    assert!(line(Square::C3, Square::C5).contains(Square::C8));
    assert_eq!(line(Square::E4, Square::E4), Bitboard::EMPTY);
    assert_eq!(
        between(Square::H1, Square::A8),
        between(Square::A8, Square::H1)
    );
}
