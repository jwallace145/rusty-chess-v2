//! Head-to-head match settings, fastchess arguments and the report.

use rusty_chess_tools::matchup::{
    Limit, Meta, Settings, Side, fastchess_args, headline, render, verdict,
};
use rusty_chess_tools::sprt::{MatchResult, parse_match};

const LOG: &str = "\
Started game 1 of 400 (candidate vs baseline)
--------------------------------------------------
Results of candidate vs baseline (10+0.1, 1t, 64MB, openings.epd):
Elo: 120.50 +/- 30.25, nElo: 150.00 +/- 35.00
LOS: 100.00 %, DrawRatio: 30.00 %, PairsRatio: 3.50
Games: 400, Wins: 210, Losses: 80, Draws: 110, Points: 265.0 (66.25 %)
Ptnml(0-2): [5, 20, 60, 70, 45], WL/DD Ratio: 1.20
--------------------------------------------------
Tournament finished
";

fn meta() -> Meta {
    Meta {
        candidate: Side {
            label: "v0.18.0".into(),
            sha: "ce7b1e2aaaaaaaa".into(),
            signature: Some(1_991_209),
        },
        baseline: Side {
            label: "v0.13.0".into(),
            sha: "f3ef9abbbbbbbbb".into(),
            signature: None,
        },
        cpu: "AMD EPYC 9V74".into(),
        fastchess: "v1.8.2-alpha".into(),
        openings: 300,
        note: "cumulative check after Phase 4".into(),
    }
}

fn result(elo: f64, err: f64) -> MatchResult {
    MatchResult {
        elo,
        elo_error: err,
        los: 50.0,
        games: 100,
        wins: 40,
        losses: 30,
        draws: 30,
        pentanomial: None,
    }
}

#[test]
fn settings_validate_and_pick_the_limit() {
    let s = Settings::new("10+0.1", None, 400, 4, 64, "o.epd").unwrap();
    assert_eq!(s.limit, Limit::Clock("10+0.1".into()));
    let s = Settings::new("ignored", Some(200_000), 400, 4, 64, "o.epd").unwrap();
    assert_eq!(s.limit, Limit::Nodes(200_000));
    assert!(Settings::new("40/60+0.6", None, 2, 1, 1, "o.epd").is_ok());
    assert!(Settings::new("60", None, 2, 1, 1, "o.epd").is_ok());
    for bad_tc in ["", "fast", "10+", "+0.1", "0/10+1", "10+-1"] {
        assert!(
            Settings::new(bad_tc, None, 2, 1, 1, "o.epd").is_err(),
            "{bad_tc}"
        );
    }
    assert!(Settings::new("10+0.1", Some(0), 2, 1, 1, "o.epd").is_err());
    assert!(
        Settings::new("10+0.1", None, 3, 1, 1, "o.epd").is_err(),
        "odd"
    );
    assert!(Settings::new("10+0.1", None, 0, 1, 1, "o.epd").is_err());
    assert!(Settings::new("10+0.1", None, 2, 0, 1, "o.epd").is_err());
    assert!(Settings::new("10+0.1", None, 2, 1, 0, "o.epd").is_err());
}

#[test]
fn fastchess_arguments_follow_the_limit() {
    let s = Settings::new("10+0.1", None, 400, 4, 64, "o.epd").unwrap();
    let a = fastchess_args(&s).join(" ");
    assert!(
        a.starts_with("-each tc=10+0.1 option.Hash=64 -openings file=o.epd"),
        "{a}"
    );
    assert!(!a.contains("nodes="), "{a}");
    assert!(
        a.contains("-rounds 200") && a.contains("-games 2 -repeat") && a.contains("-concurrency 4")
    );
    let s = Settings::new("10+0.1", Some(200_000), 100, 2, 16, "o.epd").unwrap();
    let a = fastchess_args(&s).join(" ");
    assert!(
        a.starts_with("-each tc=600+6 nodes=200000 option.Hash=16"),
        "{a}"
    );
    assert!(a.contains("-rounds 50"), "{a}");
}

#[test]
fn the_verdict_reads_the_interval() {
    assert!(verdict(&result(50.0, 20.0)).contains("stronger"));
    assert!(verdict(&result(-50.0, 20.0)).contains("weaker"));
    assert!(verdict(&result(10.0, 20.0)).contains("no significant difference"));
    assert!(verdict(&result(f64::INFINITY, f64::NAN)).contains("won every game"));
}

#[test]
fn the_report_names_both_engines_and_the_settings() {
    let r = parse_match(LOG).unwrap();
    let s = Settings::new("10+0.1", None, 400, 4, 64, "tests/openings/openings.epd").unwrap();
    let md = render(&r, &s, &meta());
    assert!(
        md.starts_with("### ⚔️ Match: `v0.18.0` vs `v0.13.0`"),
        "{md}"
    );
    assert!(md.contains("> cumulative check after Phase 4"), "{md}");
    assert!(md.contains("🟢 **the candidate is stronger"), "{md}");
    assert!(
        md.contains("| +120.5 ± 30.2 | 100.0% | 400 | 210 / 80 / 110 | 66.2% |"),
        "{md}"
    );
    assert!(
        md.contains("| Candidate | `v0.18.0` | `ce7b1e2` | 1,991,209 |"),
        "{md}"
    );
    assert!(
        md.contains("| Baseline | `v0.13.0` | `f3ef9ab` | — |"),
        "{md}"
    );
    assert!(md.contains("- Limit: 10+0.1 (seconds + increment)"), "{md}");
    assert!(
        md.contains("Pentanomial (pairs: LL, LD, WL+DD, WD, WW): 5 20 60 70 45"),
        "{md}"
    );
    let line = headline(&r, &meta());
    assert!(
        line.starts_with("v0.18.0 vs v0.13.0: +120.5 ± 30.2 Elo, LOS 100.0%, 400 games"),
        "{line}"
    );
}
