//! The post-mortem's guard against evaluation asymmetry: for every bench
//! position, the mirrored position (ranks flipped, colors and side to move
//! swapped) must evaluate to exactly the same score from the mover's view.

use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;

/// The bench positions live in the binary crate, which integration tests
/// cannot import, so the FENs are read out of the source file's string
/// literals. A test below checks that the count matches the bench.
fn bench_positions() -> Vec<String> {
    include_str!("../src/positions.rs")
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            l.strip_prefix('"')
                .and_then(|rest| rest.strip_suffix("\","))
                .map(str::to_string)
        })
        .collect()
}

fn mirror(fen: &str) -> String {
    let fields: Vec<&str> = fen.split(' ').collect();
    let swap = |c: char| {
        if c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else if c.is_ascii_lowercase() {
            c.to_ascii_uppercase()
        } else {
            c
        }
    };
    let ranks: Vec<String> = fields[0]
        .split('/')
        .rev()
        .map(|r| r.chars().map(swap).collect())
        .collect();
    let side = if fields[1] == "w" { "b" } else { "w" };
    let castling: String = if fields[2] == "-" {
        "-".into()
    } else {
        let mut s: Vec<char> = fields[2].chars().map(swap).collect();
        s.sort_by_key(|c| "KQkq".find(*c).unwrap_or(9));
        s.into_iter().collect()
    };
    let ep = if fields[3] == "-" {
        "-".to_string()
    } else {
        let b = fields[3].as_bytes();
        format!("{}{}", b[0] as char, (b'9' - b[1] + b'0') as char)
    };
    format!(
        "{} {side} {castling} {ep} {} {}",
        ranks.join("/"),
        fields[4],
        fields[5]
    )
}

#[test]
fn the_bench_positions_are_all_read() {
    assert_eq!(bench_positions().len(), 50);
}

#[test]
fn every_bench_position_evaluates_the_same_as_its_mirror() {
    for fen in &bench_positions() {
        let mirrored = mirror(fen);
        let a = Board::from_position(Position::from_fen(fen).unwrap()).evaluate();
        let b = Board::from_position(
            Position::from_fen(&mirrored).unwrap_or_else(|e| panic!("{mirrored}: {e}")),
        )
        .evaluate();
        assert_eq!(
            a, b,
            "{fen} evaluates {a} but its mirror {mirrored} evaluates {b}"
        );
    }
}

#[test]
fn mirroring_twice_is_the_identity() {
    for fen in &bench_positions() {
        assert_eq!(&mirror(&mirror(fen)), fen);
    }
}
