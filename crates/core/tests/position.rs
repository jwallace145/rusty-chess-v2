//! The position: FEN in and out, Zobrist keys, attacks, make and unmake.

use rusty_chess_core::bitboard::Bitboard;
use rusty_chess_core::r#move::{Move, MoveFlag};
use rusty_chess_core::position::{CastleRights, Piece, Position};
use rusty_chess_core::types::{Color, PieceKind, Square};

const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const KIWIPETE: &str = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";

fn pos(fen: &str) -> Position {
    Position::from_fen(fen).unwrap_or_else(|e| panic!("{fen}: {e}"))
}

#[test]
fn the_start_position_parses() {
    let p = pos(START);
    assert_eq!(
        p.piece_at(Square::E1),
        Some(Piece::new(Color::White, PieceKind::King))
    );
    assert_eq!(
        p.piece_at(Square::D8),
        Some(Piece::new(Color::Black, PieceKind::Queen))
    );
    assert_eq!(p.piece_at(Square::E4), None);
    assert_eq!(p.side_to_move(), Color::White);
    assert_eq!(p.castling(), CastleRights::ALL);
    assert_eq!(p.en_passant(), None);
    assert_eq!((p.halfmove_clock(), p.fullmove_number()), (0, 1));
    assert_eq!(p.pieces(Color::White, PieceKind::Pawn), Bitboard::RANK_2);
    assert_eq!(p.occupied().count(), 32);
    assert_eq!(p.king_square(Color::Black), Square::E8);
    assert_eq!(Position::startpos(), p);
}

#[test]
fn fen_round_trips() {
    for fen in [
        START,
        KIWIPETE,
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3",
        "4k3/8/8/8/8/8/8/4K2R b K - 7 40",
    ] {
        assert_eq!(pos(fen).to_fen(), fen);
    }
}

#[test]
fn an_en_passant_square_nobody_can_capture_is_dropped() {
    // Black just played e7-e5 but no white pawn attacks e6.
    let p = pos("rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq e6 0 2");
    assert_eq!(p.en_passant(), None);
    assert_eq!(
        p.to_fen(),
        "rnbqkbnr/pppp1ppp/8/4p3/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 2"
    );
}

#[test]
fn bad_fens_are_errors() {
    for fen in [
        "",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBN w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNRR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq z3 0 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - x 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 0",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkqq - 0 1",
        "8/8/8/8/8/8/8/8 w - - 0 1",
        "k7/8/8/8/8/8/8/KK6 w - - 0 1",
    ] {
        assert!(Position::from_fen(fen).is_err(), "{fen} should not parse");
    }
}

#[test]
fn zobrist_keys_depend_on_everything_that_matters() {
    let start = pos(START);
    assert_eq!(start.key(), pos(START).key());
    assert_ne!(start.key(), 0);
    let black = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1");
    assert_ne!(start.key(), black.key());
    let no_castle = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w Qkq - 0 1");
    assert_ne!(start.key(), no_castle.key());
    let a = pos("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
    let b = pos("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq - 0 3");
    assert_ne!(a.key(), b.key());
    // The counters are not part of the key.
    let c = pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 5 9");
    assert_eq!(start.key(), c.key());
}

#[test]
fn gives_check_agrees_with_making_the_move_everywhere_in_the_perft_suite() {
    // Every legal move of every suite position, and of every position one
    // ply below it: direct checks, discovered checks, en passant, castling
    // and promotions are all in the suite.
    fn check_all(p: &mut Position, depth: u32, checked: &mut u64) {
        let mut list = rusty_chess_core::movegen::MoveList::new();
        p.legal_moves(&mut list);
        for &m in list.iter() {
            let predicted = p.gives_check(m);
            let undo = p.make_move(m);
            assert_eq!(predicted, p.in_check(), "{m} in {}", p.to_fen());
            *checked += 1;
            if depth > 1 {
                check_all(p, depth - 1, checked);
            }
            p.unmake_move(m, undo);
        }
    }
    let mut checked = 0;
    for line in include_str!("../../../tests/perft.epd").lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fen = line.split(';').next().unwrap().trim();
        check_all(&mut pos(fen), 2, &mut checked);
    }
    assert!(checked > 20_000, "{checked}");
    // A few named cases: a castling check, an en passant discovered check,
    // a promotion check and a discovered check by a king move.
    for (fen, mv, expected) in [
        ("5k2/8/8/8/8/8/8/4K2R w K - 0 1", "e1g1", true),
        // En passant clears two squares on the rank between rook and king.
        ("8/8/8/R2pP2k/8/8/8/4K3 w - d6 0 1", "e5d6", true),
        ("8/P7/8/8/8/8/8/k1K5 w - - 0 1", "a7a8q", true),
        ("8/P7/8/8/8/8/8/k1K5 w - - 0 1", "a7a8n", false),
        ("4k3/8/8/8/8/8/4K3/4R3 w - - 0 1", "e2d3", true),
    ] {
        let p = pos(fen);
        let m = p
            .parse_uci_move(mv)
            .unwrap_or_else(|| panic!("{mv} in {fen}"));
        assert_eq!(p.gives_check(m), expected, "{mv} in {fen}");
    }
}

#[test]
fn attacks_and_checkers() {
    let p = pos(KIWIPETE);
    assert!(!p.is_attacked(Square::E5, Color::Black)); // the e6 pawn blocks the queen; no knight reaches it
    assert!(p.is_attacked(Square::D5, Color::Black)); // e6 pawn
    assert!(!p.is_attacked(Square::A1, Color::Black));
    assert!(p.is_attacked(Square::H3, Color::White)); // g2 pawn
    assert_eq!(p.checkers(), Bitboard::EMPTY);
    let in_check = pos("rnb1kbnr/pppp1ppp/8/4p3/7q/5P2/PPPPP1PP/RNBQKBNR w KQkq - 1 3");
    assert_eq!(in_check.checkers(), Bitboard::from_square(Square::H4));
    assert!(in_check.in_check());
    let double = pos("4k3/8/8/8/8/3n4/8/4K2r w - - 0 1");
    assert_eq!(double.checkers().count(), 2);
}

#[test]
fn moves_pack_into_sixteen_bits() {
    assert_eq!(std::mem::size_of::<Move>(), 2);
    let m = Move::new(Square::E2, Square::E4, MoveFlag::DoublePush);
    assert_eq!((m.from(), m.to()), (Square::E2, Square::E4));
    assert_eq!(m.flag(), MoveFlag::DoublePush);
    assert!(!m.is_capture() && !m.is_promotion());
    assert_eq!(m.to_string(), "e2e4");
    let promo = Move::new(Square::B7, Square::A8, MoveFlag::PromoCaptureQueen);
    assert!(promo.is_capture() && promo.is_promotion());
    assert_eq!(promo.promotion(), Some(PieceKind::Queen));
    assert_eq!(promo.to_string(), "b7a8q");
    assert_eq!(
        Move::new(Square::E7, Square::E8, MoveFlag::PromoKnight).to_string(),
        "e7e8n"
    );
    assert!(Move::new(Square::E1, Square::G1, MoveFlag::KingCastle).is_castle());
    assert!(Move::new(Square::E5, Square::D6, MoveFlag::EnPassant).is_capture());
    assert_eq!(Move::NULL.to_string(), "0000");
}

fn play(p: &mut Position, moves: &[(Square, Square, MoveFlag)]) {
    for &(from, to, flag) in moves {
        let m = Move::new(from, to, flag);
        p.make_move(m);
        assert_eq!(p.key(), p.recompute_key(), "key drifted after {m}");
        assert_eq!(
            p.to_fen(),
            Position::from_fen(&p.to_fen()).unwrap().to_fen()
        );
    }
}

#[test]
fn make_and_unmake_restore_the_exact_position() {
    let original = pos(KIWIPETE);
    let moves = [
        (Square::E2, Square::A6, MoveFlag::Capture),
        (Square::B4, Square::B3, MoveFlag::Quiet),
        (Square::E1, Square::G1, MoveFlag::KingCastle),
        (Square::E8, Square::C8, MoveFlag::QueenCastle),
        (Square::D5, Square::E6, MoveFlag::Capture),
        (Square::H3, Square::G2, MoveFlag::Capture),
        (Square::A2, Square::A4, MoveFlag::DoublePush),
        (Square::G2, Square::F1, MoveFlag::PromoCaptureQueen),
    ];
    let mut p = original.clone();
    let mut undos = Vec::new();
    for &(from, to, flag) in &moves {
        let m = Move::new(from, to, flag);
        undos.push(p.make_move(m));
        assert_eq!(p.key(), p.recompute_key(), "key drifted after {m}");
    }
    assert_ne!(p, original);
    for (&(from, to, flag), undo) in moves.iter().rev().zip(undos.into_iter().rev()) {
        p.unmake_move(Move::new(from, to, flag), undo);
    }
    assert_eq!(p, original);
    assert_eq!(p.to_fen(), KIWIPETE);
}

#[test]
fn castling_rights_and_clocks_update() {
    let mut p = pos(KIWIPETE);
    play(&mut p, &[(Square::E1, Square::F1, MoveFlag::Quiet)]);
    assert_eq!(
        p.castling(),
        CastleRights::BLACK_KING | CastleRights::BLACK_QUEEN
    );
    assert_eq!(p.halfmove_clock(), 1);
    assert_eq!(p.fullmove_number(), 1);
    play(&mut p, &[(Square::H8, Square::H5, MoveFlag::Quiet)]);
    assert_eq!(p.castling(), CastleRights::BLACK_QUEEN);
    assert_eq!(p.fullmove_number(), 2);
    play(
        &mut p,
        &[
            (Square::E5, Square::C6, MoveFlag::Quiet),
            (Square::A8, Square::B8, MoveFlag::Quiet),
        ],
    );
    assert_eq!(p.castling(), CastleRights::NONE);
    assert_eq!(p.halfmove_clock(), 4);
    // A pawn move resets the clock.
    play(&mut p, &[(Square::D5, Square::D6, MoveFlag::Quiet)]);
    assert_eq!(p.halfmove_clock(), 0);
}

#[test]
fn capturing_a_rook_removes_that_castling_right() {
    let mut p = pos("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    play(&mut p, &[(Square::A1, Square::A8, MoveFlag::Capture)]);
    assert_eq!(
        p.castling(),
        CastleRights::WHITE_KING | CastleRights::BLACK_KING
    );
    assert_eq!(p.halfmove_clock(), 0);
}

#[test]
fn en_passant_moves_the_captured_pawn_off_the_board() {
    let mut p = pos("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
    assert_eq!(p.en_passant(), Some(Square::D6));
    let m = Move::new(Square::E5, Square::D6, MoveFlag::EnPassant);
    let undo = p.make_move(m);
    assert_eq!(p.piece_at(Square::D5), None);
    assert_eq!(
        p.piece_at(Square::D6),
        Some(Piece::new(Color::White, PieceKind::Pawn))
    );
    assert_eq!(p.en_passant(), None);
    assert_eq!(p.key(), p.recompute_key());
    p.unmake_move(m, undo);
    assert_eq!(
        p.to_fen(),
        "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3"
    );
}

#[test]
fn a_double_push_sets_en_passant_only_when_capturable() {
    let mut p = pos(START);
    play(&mut p, &[(Square::E2, Square::E4, MoveFlag::DoublePush)]);
    assert_eq!(p.en_passant(), None);
    let mut p = pos("rnbqkbnr/pppp1ppp/8/8/4p3/8/PPPPPPPP/RNBQKBNR w KQkq - 0 3");
    play(&mut p, &[(Square::D2, Square::D4, MoveFlag::DoublePush)]);
    assert_eq!(p.en_passant(), Some(Square::D3));
    assert!(p.to_fen().contains(" d3 "));
}

#[test]
fn promotions_replace_the_pawn() {
    let mut p = pos("4k3/1P6/8/8/8/8/8/4K3 w - - 0 1");
    let m = Move::new(Square::B7, Square::B8, MoveFlag::PromoKnight);
    let undo = p.make_move(m);
    assert_eq!(
        p.piece_at(Square::B8),
        Some(Piece::new(Color::White, PieceKind::Knight))
    );
    assert_eq!(p.pieces(Color::White, PieceKind::Pawn), Bitboard::EMPTY);
    p.unmake_move(m, undo);
    assert_eq!(
        p.piece_at(Square::B7),
        Some(Piece::new(Color::White, PieceKind::Pawn))
    );
    assert_eq!(p.piece_at(Square::B8), None);
}

#[test]
fn a_null_move_passes_the_turn_and_unmakes_exactly() {
    let original = pos("rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 3");
    let mut p = original.clone();
    let undo = p.make_null_move();
    assert_eq!(p.side_to_move(), Color::Black);
    assert_eq!(p.en_passant(), None, "the en passant right lapses");
    assert_eq!(p.halfmove_clock(), 1);
    assert_eq!(p.fullmove_number(), 3);
    assert_eq!(p.key(), p.recompute_key());
    assert_ne!(p.key(), original.key());
    assert_eq!(p.piece_at(Square::E5), original.piece_at(Square::E5));
    p.unmake_null_move(undo);
    assert_eq!(p, original);
    // Black's null move advances the move number.
    let mut p = pos("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1");
    let undo = p.make_null_move();
    assert_eq!(p.fullmove_number(), 2);
    p.unmake_null_move(undo);
    assert_eq!(p.fullmove_number(), 1);
}

#[test]
fn non_pawn_material() {
    assert!(
        pos("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .has_non_pawn_material(Color::White)
    );
    let pawns_only = pos("8/k7/3p4/p2P1p2/P2P1P2/8/8/K7 w - - 0 1");
    assert!(!pawns_only.has_non_pawn_material(Color::White));
    assert!(!pawns_only.has_non_pawn_material(Color::Black));
    let one_knight = pos("8/k7/3p4/p2P1p2/P2P1P2/8/8/K5N1 w - - 0 1");
    assert!(one_knight.has_non_pawn_material(Color::White));
    assert!(!one_knight.has_non_pawn_material(Color::Black));
}
