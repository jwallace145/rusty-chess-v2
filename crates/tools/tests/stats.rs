//! Pentanomial interval estimates, checked against fastchess's own numbers.

use rusty_chess_tools::stats::{Confidence, elo_of, from_pentanomial, games_for, normal_cdf};

#[test]
fn the_95_percent_interval_matches_fastchess_on_real_runs() {
    // (pentanomial, fastchess's Elo, its 95% error) from SPRT comments:
    // #90 timed (IIR), #90 fixed nodes, #81's first form timed.
    for (p, elo, err) in [
        ([27, 100, 188, 168, 35], 28.2, 14.8),
        ([46, 230, 346, 214, 102], 17.8, 11.7),
        ([66, 240, 364, 254, 76], 5.9, 11.1),
    ] {
        let e = from_pentanomial(p, Confidence::P95).unwrap();
        assert!((e.elo - elo).abs() < 0.15, "{p:?}: elo {} vs {elo}", e.elo);
        let half = (e.upper - e.lower) / 2.0;
        assert!((half - err).abs() < 0.3, "{p:?}: ±{half} vs ±{err}");
    }
}

#[test]
fn higher_confidence_widens_the_interval() {
    let p = [27, 100, 188, 168, 35];
    let w = |c| {
        let e = from_pentanomial(p, c).unwrap();
        e.upper - e.lower
    };
    assert!(w(Confidence::P95) < w(Confidence::P99));
    assert!(w(Confidence::P99) < w(Confidence::P999));
    // The width scales with z: 99% is about 1.31 times 95%.
    let ratio = w(Confidence::P99) / w(Confidence::P95);
    assert!((ratio - 2.575_829 / 1.959_964).abs() < 0.02, "{ratio}");
}

#[test]
fn los_is_the_chance_the_first_engine_is_stronger() {
    let even = from_pentanomial([10, 40, 100, 40, 10], Confidence::P99).unwrap();
    assert!((even.los - 50.0).abs() < 1e-4 && even.elo.abs() < 1e-9);
    let ahead = from_pentanomial([27, 100, 188, 168, 35], Confidence::P99).unwrap();
    assert!(ahead.los > 99.9, "{}", ahead.los);
    let behind = from_pentanomial([35, 168, 188, 100, 27], Confidence::P99).unwrap();
    assert!((behind.los + ahead.los - 100.0).abs() < 1e-4);
    assert!((normal_cdf(1.959_964) - 0.975).abs() < 1e-6);
}

#[test]
fn one_side_scoring_everything_is_infinite_and_too_few_pairs_is_nothing() {
    let all = from_pentanomial([0, 0, 0, 0, 20], Confidence::P99).unwrap();
    assert!(all.elo.is_infinite() && all.elo > 0.0 && all.los == 100.0);
    assert!(from_pentanomial([0, 0, 1, 0, 0], Confidence::P99).is_none());
    assert_eq!(elo_of(0.5), 0.0);
    assert!(elo_of(0.0).is_infinite());
}

#[test]
fn games_needed_scale_with_the_square_of_the_precision() {
    let e = from_pentanomial([27, 100, 188, 168, 35], Confidence::P95).unwrap();
    // This match played 1036 games for about ±14.8; the estimate agrees.
    let now = games_for(&e, 14.8).unwrap();
    assert!((900..=1200).contains(&now), "{now}");
    let half = games_for(&e, 7.4).unwrap();
    assert!(
        (half as f64 / now as f64 - 4.0).abs() < 0.1,
        "{half} vs {now}"
    );
    assert_eq!(games_for(&e, 0.0), None);
}

#[test]
fn confidence_parses_the_three_levels() {
    assert_eq!(Confidence::parse("99").unwrap(), Confidence::P99);
    assert_eq!(Confidence::parse("99.9%").unwrap(), Confidence::P999);
    assert_eq!(Confidence::parse(" 95 ").unwrap(), Confidence::P95);
    assert!(Confidence::parse("90").is_err());
}
