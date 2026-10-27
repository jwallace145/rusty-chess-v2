//! The rating ladder: parsing non-SPRT match logs, combining levels, bounds.

use rusty_chess_tools::gate::{Rating, Targets};
use rusty_chess_tools::rating::{
    Level, Meta, estimate, fastchess_args, headline, render, render_json,
};
use rusty_chess_tools::sprt::{MatchResult, parse_match};

fn level(elo: u32, diff: f64, err: f64, w: u32, l: u32, d: u32) -> Level {
    Level {
        elo,
        result: MatchResult {
            elo: diff,
            elo_error: err,
            los: 50.0,
            games: w + l + d,
            wins: w,
            losses: l,
            draws: d,
            pentanomial: None,
        },
    }
}

#[test]
fn a_plain_match_log_parses_without_an_sprt() {
    let log = "Started game 1 of 100 (rusty-chess vs stockfish-1800)\n--------------------------------------------------\nResults of rusty-chess vs stockfish-1800 (10+0.1, NULL, 64MB, openings.epd):\nElo: 120.50 +/- 45.20, nElo: 150.00 +/- 50.00\nLOS: 100.00 %, DrawRatio: 20.00 %, PairsRatio: 3.00\nGames: 100, Wins: 60, Losses: 27, Draws: 13, Points: 66.5 (66.50 %)\nPtnml(0-2): [2, 5, 10, 15, 18], WL/DD Ratio: 2.00\n--------------------------------------------------\n";
    let m = parse_match(log).unwrap();
    assert_eq!((m.elo, m.elo_error), (120.5, 45.2));
    assert_eq!((m.games, m.wins, m.losses, m.draws), (100, 60, 27, 13));
    assert!((m.score() - 0.665).abs() < 1e-9);
    // A clean sweep reports an infinite difference.
    let sweep = log.replace("Elo: 120.50 +/- 45.20", "Elo: inf +/- nan");
    assert!(parse_match(&sweep).unwrap().elo.is_infinite());
}

#[test]
fn levels_combine_by_inverse_variance() {
    // 1800 + 120 = 1920 with a tight bar; 2100 − 150 = 1950 with a wide one.
    let e = estimate(&[
        level(1800, 120.0, 40.0, 60, 27, 13),
        level(2100, -150.0, 80.0, 25, 60, 15),
    ]);
    let r = e.rating.unwrap();
    assert!(r > 1920.0 && r < 1935.0, "closer to the precise level: {r}");
    assert!(
        e.error.unwrap() < 40.0,
        "combining narrows the bar: {:?}",
        e.error
    );
    assert_eq!(headline(&e), format!("~{:.0} ± {:.0}", r, e.error.unwrap()));
}

#[test]
fn sweeps_only_bound_the_rating() {
    let e = estimate(&[
        level(1500, f64::INFINITY, f64::NAN, 100, 0, 0),
        level(2400, f64::NEG_INFINITY, f64::NAN, 0, 100, 0),
    ]);
    assert_eq!(
        (e.rating, e.floor, e.ceiling),
        (None, Some(1500), Some(2400))
    );
    assert_eq!(headline(&e), "between 1500 and 2400");
    let e = estimate(&[
        level(1500, f64::INFINITY, f64::NAN, 100, 0, 0),
        level(1800, 50.0, 60.0, 50, 36, 14),
    ]);
    assert!(e.rating.is_some() && e.floor == Some(1500));
}

#[test]
fn settings_and_arguments() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[rating]\nlevels = [1600, 2000]\ngames_per_level = 41\n").unwrap();
    assert_eq!(t.rating.levels, vec![1600, 2000]);
    assert_eq!(t.rating.tc, Rating::default().tc);
    let args = fastchess_args(&t.rating, t.rating.games_per_level).join(" ");
    assert!(
        args.contains("-each tc=10+0.1")
            && args.contains("-rounds 21")
            && args.contains("-games 2 -repeat"),
        "{args}"
    );
}

#[test]
fn the_report_lists_every_level_and_the_json_is_parseable() {
    let levels = [
        level(1500, f64::INFINITY, f64::NAN, 100, 0, 0),
        level(1800, 120.0, 40.0, 60, 27, 13),
        level(2100, -150.0, 80.0, 25, 60, 15),
    ];
    let e = estimate(&levels);
    let meta = Meta {
        version: "v0.7.0".into(),
        stockfish: "Stockfish 16".into(),
        cpu: "AMD EPYC 7763".into(),
    };
    let md = render(&levels, &e, &Rating::default(), &meta);
    assert!(md.starts_with("## Strength\n"), "{md}");
    assert!(
        md.contains("| 1500 | 100 | 100 / 0 / 0 | 100.0% | +∞ | > 1500 |"),
        "{md}"
    );
    assert!(
        md.contains("| 1800 | 100 | 60 / 27 / 13 | 66.5% | +120 ± 40 | 1920 |"),
        "{md}"
    );
    assert!(
        md.contains("| 2100 | 100 | 25 / 60 / 15 | 32.5% | −150 ± 80 | 1950 |"),
        "{md}"
    );
    assert!(md.contains("Stockfish 16 on AMD EPYC 7763"));
    let v: serde_json::Value =
        serde_json::from_str(&render_json(&levels, &e, &Rating::default(), &meta)).unwrap();
    assert_eq!(v["version"], "v0.7.0");
    assert_eq!(v["levels"][0]["elo_diff"], serde_json::Value::Null);
    assert_eq!(v["levels"][1]["stockfish_elo"], 1800);
    assert!(v["rating"].as_f64().unwrap() > 1900.0);
}
