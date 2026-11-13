//! Lab measurement: what the default net thinks a piece is worth. For each
//! position (FENs on stdin, one per line), every non-king piece is removed in
//! turn and the static evaluation re-read from the side to move's view; the
//! change, signed for the piece's owner, is that piece's value there.
use std::io::BufRead;

use rusty_chess_core::position::Position;
use rusty_chess_core::types::{PieceKind, Square};
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::default_network;

fn eval(fen: &str) -> Option<i32> {
    let pos = Position::from_fen(fen).ok()?;
    if pos.in_check() {
        return None;
    }
    let mut b = Board::from_position(pos);
    // PESTO=1 measures the hand-written evaluation instead, for comparison.
    if std::env::var("PESTO").is_err() {
        b.set_network(Some(default_network()));
    }
    Some(b.evaluate())
}

fn main() {
    let mut sum = [0i64; 5];
    let mut sq = [0i64; 5];
    let mut n = [0i64; 5];
    let mut all: [Vec<i32>; 5] = Default::default();
    for line in std::io::stdin().lock().lines() {
        let line = line.unwrap();
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 4 {
            continue;
        }
        let fen = format!(
            "{} {} {} {} 0 1",
            fields[0], fields[1], fields[2], fields[3]
        );
        let Ok(pos) = Position::from_fen(&fen) else {
            continue;
        };
        let Some(base) = eval(&fen) else { continue };
        let stm = pos.side_to_move();
        // Rebuild the placement field with one piece removed.
        let rows: Vec<Vec<char>> = fields[0]
            .split('/')
            .map(|r| {
                let mut out = Vec::new();
                for c in r.chars() {
                    if let Some(d) = c.to_digit(10) {
                        for _ in 0..d {
                            out.push('.');
                        }
                    } else {
                        out.push(c);
                    }
                }
                out
            })
            .collect();
        for s in Square::ALL {
            let Some(p) = pos.piece_at(s) else { continue };
            if p.kind == PieceKind::King {
                continue;
            }
            let (file, rank) = (s.index() % 8, s.index() / 8);
            let mut rows2 = rows.clone();
            rows2[7 - rank][file] = '.';
            let placement: Vec<String> = rows2
                .iter()
                .map(|r| {
                    let mut out = String::new();
                    let mut run = 0;
                    for &c in r {
                        if c == '.' {
                            run += 1;
                        } else {
                            if run > 0 {
                                out.push_str(&run.to_string());
                                run = 0;
                            }
                            out.push(c);
                        }
                    }
                    if run > 0 {
                        out.push_str(&run.to_string());
                    }
                    out
                })
                .collect();
            // Castling rights dropped: a removed rook would make them illegal.
            let fen2 = format!("{} {} - - 0 1", placement.join("/"), fields[1]);
            let Some(without) = eval(&fen2) else { continue };
            // The side to move's evaluation falls when its own piece goes.
            let value = if p.color == stm {
                base - without
            } else {
                without - base
            };
            let k = p.kind.index();
            sum[k] += i64::from(value);
            sq[k] += i64::from(value) * i64::from(value);
            n[k] += 1;
            all[k].push(value);
        }
    }
    for (k, name) in ["pawn", "knight", "bishop", "rook", "queen"]
        .iter()
        .enumerate()
    {
        if n[k] == 0 {
            continue;
        }
        let mean = sum[k] as f64 / n[k] as f64;
        let sd = (sq[k] as f64 / n[k] as f64 - mean * mean).sqrt();
        all[k].sort_unstable();
        let q = |f: f64| all[k][((all[k].len() - 1) as f64 * f) as usize];
        println!(
            "{name:7} n={:6} mean={mean:7.1} sd={sd:6.1} p10={:5} median={:5} p90={:5}",
            n[k],
            q(0.1),
            q(0.5),
            q(0.9)
        );
    }
}
