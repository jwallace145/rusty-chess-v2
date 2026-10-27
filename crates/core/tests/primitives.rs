//! Squares, colors, pieces and bitboards.

use rusty_chess_core::bitboard::Bitboard;
use rusty_chess_core::types::{Color, PieceKind, Square};

#[test]
fn squares_index_a1_to_h8_file_first() {
    assert_eq!(Square::A1.index(), 0);
    assert_eq!(Square::H1.index(), 7);
    assert_eq!(Square::A2.index(), 8);
    assert_eq!(Square::H8.index(), 63);
    assert_eq!(Square::new(4, 1), Square::E2);
    assert_eq!((Square::E2.file(), Square::E2.rank()), (4, 1));
    assert_eq!(Square::from_index(36), Square::E5);
}

#[test]
fn squares_parse_and_print_algebraic_names() {
    assert_eq!("e4".parse::<Square>(), Ok(Square::E4));
    assert_eq!(Square::E4.to_string(), "e4");
    assert!("e9".parse::<Square>().is_err());
    assert!("i1".parse::<Square>().is_err());
    assert!("e".parse::<Square>().is_err());
    for i in 0..64u8 {
        let sq = Square::from_index(i);
        assert_eq!(sq.to_string().parse::<Square>(), Ok(sq));
    }
}

#[test]
fn square_arithmetic() {
    assert_eq!(Square::E2.offset(0, 2), Some(Square::E4));
    assert_eq!(Square::A1.offset(-1, 0), None);
    assert_eq!(Square::H8.offset(1, 1), None);
    assert_eq!(Square::E4.flip_rank(), Square::E5);
    assert_eq!(Square::A1.flip_rank(), Square::A8);
}

#[test]
fn colors_and_pieces() {
    assert_eq!(!Color::White, Color::Black);
    assert_eq!(Color::Black.flip(), Color::White);
    assert_eq!(Color::White.index(), 0);
    assert_eq!(PieceKind::ALL.len(), 6);
    assert_eq!(PieceKind::Queen.index(), 4);
    assert_eq!(PieceKind::from_char('n'), Some(PieceKind::Knight));
    assert_eq!(PieceKind::from_char('x'), None);
    assert_eq!(PieceKind::Rook.to_char(Color::Black), 'r');
    assert_eq!(PieceKind::Rook.to_char(Color::White), 'R');
}

#[test]
fn bitboard_set_operations_and_iteration() {
    let bb = Bitboard::from_square(Square::A1) | Bitboard::from_square(Square::H8);
    assert_eq!(bb.count(), 2);
    assert!(bb.contains(Square::A1));
    assert!(!bb.contains(Square::A2));
    assert_eq!(bb.lsb(), Some(Square::A1));
    assert_eq!(
        bb.into_iter().collect::<Vec<_>>(),
        vec![Square::A1, Square::H8]
    );
    let mut b = bb;
    assert_eq!(b.pop_lsb(), Some(Square::A1));
    assert_eq!(b, Bitboard::from_square(Square::H8));
    assert_eq!(b.pop_lsb(), Some(Square::H8));
    assert_eq!(b.pop_lsb(), None);
    assert!(b.is_empty());
    assert_eq!(!Bitboard::EMPTY, Bitboard::ALL);
    assert_eq!((Bitboard::ALL ^ Bitboard::ALL), Bitboard::EMPTY);
    assert_eq!(Bitboard::ALL.without(Square::E4).count(), 63);
    assert_eq!(
        Bitboard::EMPTY.with(Square::E4),
        Bitboard::from_square(Square::E4)
    );
}

#[test]
fn bitboard_files_ranks_and_shifts() {
    assert_eq!(Bitboard::FILE_A.count(), 8);
    assert!(Bitboard::FILE_A.contains(Square::A8));
    assert!(Bitboard::RANK_4.contains(Square::H4));
    assert_eq!(Bitboard::file(7), Bitboard::FILE_H);
    assert_eq!(Bitboard::rank(0), Bitboard::RANK_1);
    let e4 = Bitboard::from_square(Square::E4);
    assert_eq!(e4.north(), Bitboard::from_square(Square::E5));
    assert_eq!(e4.south(), Bitboard::from_square(Square::E3));
    assert_eq!(e4.east(), Bitboard::from_square(Square::F4));
    assert_eq!(e4.west(), Bitboard::from_square(Square::D4));
    assert_eq!(Bitboard::from_square(Square::H4).east(), Bitboard::EMPTY);
    assert_eq!(Bitboard::from_square(Square::A4).west(), Bitboard::EMPTY);
    assert_eq!(Bitboard::RANK_8.north(), Bitboard::EMPTY);
    assert_eq!(e4.shift(Color::White), e4.north());
    assert_eq!(e4.shift(Color::Black), e4.south());
}

#[test]
fn bitboard_display_is_a_board_from_whites_side() {
    let text = Bitboard::from_square(Square::A8)
        .with(Square::H1)
        .to_string();
    let rows: Vec<&str> = text.lines().collect();
    assert_eq!(rows.len(), 8);
    assert!(rows[0].starts_with('X'), "{text}");
    assert!(rows[7].ends_with('X'), "{text}");
}
