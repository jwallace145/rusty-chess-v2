//! Static exchange evaluation: what a capture sequence on one square is
//! worth in material when each side keeps capturing only while it pays.
//! Used to split captures into good (searched early) and bad (searched last).

use rusty_chess_core::attacks;
use rusty_chess_core::bitboard::Bitboard;
use rusty_chess_core::r#move::Move;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};

/// Piece values for exchanges, indexed by [`PieceKind`]. The king is worth
/// more than everything else so it is never worth "losing" in a sequence.
pub const SEE_VALUE: [i32; 6] = [100, 320, 330, 500, 900, 20_000];

/// The material outcome of `m`, which must be legal, from the mover's view.
/// Quiet moves score zero; promotions add the promotion gain.
pub fn see(pos: &Position, m: Move) -> i32 {
    if m.is_castle() {
        return 0;
    }
    let us = pos.side_to_move();
    let (from, to) = (m.from(), m.to());
    let attacker = pos
        .piece_at(from)
        .expect("see of a move from an empty square")
        .kind;

    let mut gain = [0i32; 32];
    let mut occ = pos.occupied().without(from);
    gain[0] = if m.is_en_passant() {
        let captured = to
            .offset(0, if us == Color::White { -1 } else { 1 })
            .expect("en passant square");
        occ = occ.without(captured);
        SEE_VALUE[PieceKind::Pawn.index()]
    } else {
        pos.piece_at(to).map_or(0, |p| SEE_VALUE[p.kind.index()])
    };
    // The piece that now stands on `to` and will be captured next.
    let mut on_square = match m.promotion() {
        Some(promo) => {
            gain[0] += SEE_VALUE[promo.index()] - SEE_VALUE[PieceKind::Pawn.index()];
            promo
        }
        None => attacker,
    };

    let mut side = !us;
    let mut depth = 0usize;
    let mut attackers = pos.attackers_to(to, occ) & occ;
    loop {
        let own = attackers & pos.by_color(side);
        if own.is_empty() || depth + 1 >= gain.len() {
            break;
        }
        // Least valuable attacker first.
        let (kind, sq) = PieceKind::ALL
            .iter()
            .find_map(|&k| (own & pos.by_kind(k)).lsb().map(|sq| (k, sq)))
            .expect("an attacker exists");
        if kind == PieceKind::King && (attackers & pos.by_color(!side)).is_nonempty() {
            break; // a king may not capture into an attacked square
        }
        depth += 1;
        gain[depth] = SEE_VALUE[on_square.index()] - gain[depth - 1];
        on_square = kind;
        occ = occ.without(sq);
        // Sliders behind the piece just removed now attack through it.
        attackers = pos.attackers_to(to, occ) & occ;
        side = !side;
    }
    while depth > 0 {
        gain[depth - 1] = -(-gain[depth - 1]).max(gain[depth]);
        depth -= 1;
    }
    gain[0]
}

/// Whether the exchange on `m` is worth at least `threshold`.
#[inline]
pub fn see_ge(pos: &Position, m: Move, threshold: i32) -> bool {
    see(pos, m) >= threshold
}

/// Every square attacked by `color` given `occ`, for callers that need it.
#[allow(dead_code)]
fn attacked_by(pos: &Position, color: Color, occ: Bitboard) -> Bitboard {
    let mut out = Bitboard::EMPTY;
    for sq in pos.by_color(color) {
        let kind = pos.piece_at(sq).expect("occupied").kind;
        out |= match kind {
            PieceKind::Pawn => attacks::pawn_attacks(color, sq),
            PieceKind::Knight => attacks::knight_attacks(sq),
            PieceKind::Bishop => attacks::bishop_attacks(sq, occ),
            PieceKind::Rook => attacks::rook_attacks(sq, occ),
            PieceKind::Queen => attacks::queen_attacks(sq, occ),
            PieceKind::King => attacks::king_attacks(sq),
        };
    }
    out
}
