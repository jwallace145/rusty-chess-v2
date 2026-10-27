//! The rating ladder: a strength estimate from matches against Stockfish
//! limited to fixed `UCI_Elo` levels.
//!
//! Each level's match gives an Elo difference `d ± e` (95%), so the engine's
//! rating from that level is `level + d`. Levels combine by inverse variance,
//! so the closely matched levels, which say the most, count the most. A level
//! the engine wins or loses every game against has no finite difference; it
//! only bounds the rating from below or above.
//!
//! Stockfish's `UCI_Elo` scale is calibrated to its own rating lists at long
//! time controls, so the result is a ballpark, not a CCRL rating.

use std::fmt::Write as _;

use crate::gate::Rating;
use crate::sprt::MatchResult;

/// One level's match.
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    /// Stockfish's `UCI_Elo`.
    pub elo: u32,
    /// The match from our engine's side.
    pub result: MatchResult,
}

/// The combined estimate.
#[derive(Clone, Debug, PartialEq)]
pub struct Estimate {
    /// The rating, if any level gave a finite difference.
    pub rating: Option<f64>,
    /// Its 95% error bar.
    pub error: Option<f64>,
    /// The highest level beaten in every game, a lower bound.
    pub floor: Option<u32>,
    /// The lowest level lost to in every game, an upper bound.
    pub ceiling: Option<u32>,
}

/// Combine the levels.
pub fn estimate(levels: &[Level]) -> Estimate {
    let mut weight = 0.0;
    let mut sum = 0.0;
    let mut floor = None;
    let mut ceiling = None;
    for l in levels {
        let score = l.result.score();
        if l.result.games > 0 && score >= 1.0 {
            floor = floor.max(Some(l.elo));
            continue;
        }
        if l.result.games > 0 && score <= 0.0 {
            ceiling = Some(ceiling.map_or(l.elo, |c: u32| c.min(l.elo)));
            continue;
        }
        let (d, e) = (l.result.elo, l.result.elo_error);
        if !d.is_finite() || !e.is_finite() || e <= 0.0 {
            continue;
        }
        let sigma = e / 1.96;
        let w = 1.0 / (sigma * sigma);
        weight += w;
        sum += w * (l.elo as f64 + d);
    }
    let (rating, error) = if weight > 0.0 {
        (Some(sum / weight), Some(1.96 / weight.sqrt()))
    } else {
        (None, None)
    };
    Estimate {
        rating,
        error,
        floor,
        ceiling,
    }
}

/// The fastchess arguments for one level, excluding the two engines.
pub fn fastchess_args(r: &Rating, games: u32) -> Vec<String> {
    let rounds = games.div_ceil(2).max(1);
    [
        "-each".to_string(),
        format!("tc={}", r.tc),
        "-openings".into(),
        format!("file={}", r.openings),
        "format=epd".into(),
        "order=random".into(),
        "-games".into(),
        "2".into(),
        "-repeat".into(),
        "-rounds".into(),
        rounds.to_string(),
        "-concurrency".into(),
        r.concurrency.to_string(),
        "-maxmoves".into(),
        "200".into(),
        "-recover".into(),
        "-ratinginterval".into(),
        "20".into(),
    ]
    .to_vec()
}

/// Labels for the report.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// What was rated: a tag, branch or SHA.
    pub version: String,
    /// The Stockfish version string.
    pub stockfish: String,
    /// The CPU model of the runner.
    pub cpu: String,
}

fn fmt_signed(v: f64) -> String {
    if v.is_infinite() {
        (if v > 0.0 { "+∞" } else { "−∞" }).into()
    } else if v >= 0.0 {
        format!("+{v:.0}")
    } else {
        format!("−{:.0}", -v)
    }
}

/// The one-line headline: `~2050 ± 60`, or a bound when that is all there is.
pub fn headline(e: &Estimate) -> String {
    match (e.rating, e.error) {
        (Some(r), Some(err)) => format!("~{r:.0} ± {err:.0}"),
        _ => match (e.floor, e.ceiling) {
            (Some(f), Some(c)) => format!("between {f} and {c}"),
            (Some(f), None) => format!("above {f}"),
            (None, Some(c)) => format!("below {c}"),
            (None, None) => "not measured".into(),
        },
    }
}

/// The Markdown report, for release notes and the job summary.
pub fn render(levels: &[Level], e: &Estimate, r: &Rating, meta: &Meta) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "## Strength");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Estimated rating: **{}** against Stockfish's `UCI_Elo` scale at {}.",
        headline(e),
        r.tc
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Stockfish level | Games | W / L / D | Score | Elo diff | Rating from level |"
    );
    let _ = writeln!(out, "|--:|--:|:-:|--:|--:|--:|");
    for l in levels {
        let m = &l.result;
        let from_level = if m.elo.is_finite() {
            format!("{:.0}", l.elo as f64 + m.elo)
        } else if m.elo > 0.0 {
            format!("> {}", l.elo)
        } else {
            format!("< {}", l.elo)
        };
        let diff = if m.elo.is_finite() && m.elo_error.is_finite() {
            format!("{} ± {:.0}", fmt_signed(m.elo), m.elo_error)
        } else {
            fmt_signed(m.elo)
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} / {} / {} | {:.1}% | {} | {} |",
            l.elo,
            m.games,
            m.wins,
            m.losses,
            m.draws,
            m.score() * 100.0,
            diff,
            from_level
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "{} on {}; {} games per level from `{}`, each opening from both sides. Levels combine by inverse variance. Limited Stockfish is a ballpark scale, not a CCRL rating.",
        meta.stockfish, meta.cpu, r.games_per_level, r.openings
    );
    out
}

/// The machine-readable report.
pub fn render_json(levels: &[Level], e: &Estimate, r: &Rating, meta: &Meta) -> String {
    let value = serde_json::json!({
        "v": 1,
        "version": meta.version,
        "stockfish": meta.stockfish,
        "cpu": meta.cpu,
        "tc": r.tc,
        "rating": e.rating,
        "error": e.error,
        "floor": e.floor,
        "ceiling": e.ceiling,
        "levels": levels.iter().map(|l| serde_json::json!({
            "stockfish_elo": l.elo,
            "games": l.result.games,
            "wins": l.result.wins,
            "losses": l.result.losses,
            "draws": l.result.draws,
            "score": l.result.score(),
            "elo_diff": if l.result.elo.is_finite() { serde_json::json!(l.result.elo) } else { serde_json::Value::Null },
            "elo_error": if l.result.elo_error.is_finite() { serde_json::json!(l.result.elo_error) } else { serde_json::Value::Null },
        })).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&value).expect("rating serializes")
}
