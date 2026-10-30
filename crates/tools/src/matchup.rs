//! Head-to-head matches between any two builds of the engine.
//!
//! The diagnostic behind the `Match` workflow: a candidate against a
//! baseline, both built from any tag, branch or SHA, at a time control or a
//! fixed node budget, from our openings. Unlike an SPRT it plays a fixed
//! number of games and reports the Elo difference with its interval, so it
//! answers "how far apart are these two", not "is this change a gain".
//!
//! The interval is computed here from the pentanomial counts (see
//! [`crate::stats`]) at the confidence the run asked for, 99% by default,
//! with how many games a conclusive answer would take. Decide the number of
//! games before the run: stopping a fixed-length match early because it looks
//! significant inflates the false positives.

use std::fmt::Write as _;

use crate::sprt::MatchResult;
use crate::stats::{Confidence, from_pentanomial, games_for};

/// How each move is limited.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Limit {
    /// A time control in fastchess's `moves/time+increment` form, seconds,
    /// such as `10+0.1` or `60+0.6`.
    Clock(String),
    /// A fixed number of nodes per move.
    Nodes(u64),
}

/// The match settings, validated.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settings {
    /// How each move is limited.
    pub limit: Limit,
    /// Games in total; even, so every opening is played from both sides.
    pub games: u32,
    /// Games played at once.
    pub concurrency: u32,
    /// Hash per engine in MB.
    pub hash_mb: u32,
    /// The openings file, one EPD per line.
    pub openings: String,
    /// The two-sided confidence of the reported interval.
    pub confidence: Confidence,
}

impl Settings {
    /// Build and validate. `tc` is used when `nodes` is `None`.
    pub fn new(
        tc: &str,
        nodes: Option<u64>,
        games: u32,
        concurrency: u32,
        hash_mb: u32,
        openings: &str,
    ) -> Result<Settings, String> {
        let limit = match nodes {
            Some(0) => return Err("nodes must be positive".into()),
            Some(n) => Limit::Nodes(n),
            None => {
                if !valid_tc(tc) {
                    return Err(format!(
                        "time control `{tc}` is not of the form `10+0.1` or `40/60+0.6`"
                    ));
                }
                Limit::Clock(tc.to_string())
            }
        };
        if games < 2 || games % 2 != 0 {
            return Err(format!(
                "games must be an even number of at least 2 (each opening is played from both sides), got {games}"
            ));
        }
        if !(1..=64).contains(&concurrency) {
            return Err(format!("concurrency must be 1..64, got {concurrency}"));
        }
        if !(1..=4096).contains(&hash_mb) {
            return Err(format!("hash must be 1..4096 MB, got {hash_mb}"));
        }
        Ok(Settings {
            limit,
            games,
            concurrency,
            hash_mb,
            openings: openings.to_string(),
            confidence: Confidence::P99,
        })
    }

    /// The same settings at another confidence.
    pub fn with_confidence(self, confidence: Confidence) -> Settings {
        Settings { confidence, ..self }
    }

    /// The limit as people read it: `10+0.1` or `200,000 nodes per move`.
    pub fn limit_label(&self) -> String {
        match &self.limit {
            Limit::Clock(tc) => format!("{tc} (seconds + increment)"),
            Limit::Nodes(n) => format!("{} nodes per move", fmt_int(*n)),
        }
    }
}

fn valid_tc(tc: &str) -> bool {
    let (moves, rest) = match tc.split_once('/') {
        Some((m, r)) => (Some(m), r),
        None => (None, tc),
    };
    let number = |s: &str| !s.is_empty() && s.parse::<f64>().is_ok_and(|v| v >= 0.0);
    let base_ok = match rest.split_once('+') {
        Some((base, inc)) => number(base) && number(inc),
        None => number(rest),
    };
    base_ok && moves.is_none_or(|m| m.parse::<u32>().is_ok_and(|v| v > 0))
}

/// The fastchess arguments for everything but the two `-engine` blocks.
pub fn fastchess_args(s: &Settings) -> Vec<String> {
    let mut args = vec!["-each".to_string()];
    match &s.limit {
        Limit::Clock(tc) => args.push(format!("tc={tc}")),
        // A clock is still required; make it irrelevant.
        Limit::Nodes(n) => {
            args.push("tc=600+6".into());
            args.push(format!("nodes={n}"));
        }
    }
    args.push(format!("option.Hash={}", s.hash_mb));
    args.extend(
        [
            "-openings",
            &format!("file={}", s.openings),
            "format=epd",
            "order=random",
            "-games",
            "2",
            "-repeat",
            "-rounds",
            &(s.games / 2).to_string(),
            "-concurrency",
            &s.concurrency.to_string(),
            "-maxmoves",
            "200",
            "-draw",
            "movenumber=40",
            "movecount=8",
            "score=10",
            "-resign",
            "movecount=3",
            "score=800",
            "twosided=true",
            "-ratinginterval",
            "20",
            "-report",
            "penta=true",
            "-recover",
        ]
        .map(String::from),
    );
    args
}

/// One engine in the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Side {
    /// The ref asked for: tag, branch or SHA.
    pub label: String,
    /// The commit it resolved to.
    pub sha: String,
    /// The bench signature of the build, if measured.
    pub signature: Option<u64>,
}

/// Labels for the report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Meta {
    /// The engine being judged.
    pub candidate: Side,
    /// The engine it is judged against.
    pub baseline: Side,
    /// The CPU model of the runner.
    pub cpu: String,
    /// The fastchess version.
    pub fastchess: String,
    /// Openings in the file.
    pub openings: usize,
    /// A free-text note from whoever started the run.
    pub note: String,
}

/// What a match result says at the run's confidence.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    /// Elo of the candidate over the baseline; infinite when one side scored
    /// everything.
    pub elo: f64,
    /// The interval's lower end.
    pub lower: f64,
    /// Its upper end.
    pub upper: f64,
    /// Likelihood the candidate is stronger, percent (NaN when undefined).
    pub los: f64,
    /// The confidence the interval is at, as people read it.
    pub confidence: &'static str,
    /// Games for an interval of ±5 Elo at this match's variance, when known.
    pub games_for_5: Option<u64>,
    /// When inconclusive: games for the interval to exclude zero, if the
    /// difference measured here is the true one.
    pub games_to_resolve: Option<u64>,
}

impl Reading {
    /// Whether the interval excludes zero.
    pub fn conclusive(&self) -> bool {
        self.lower > 0.0 || self.upper < 0.0
    }
}

/// Read a result at `confidence`. With pentanomial counts the interval is
/// computed here; without them it falls back to fastchess's own 95% interval.
pub fn read(r: &MatchResult, confidence: Confidence) -> Reading {
    if let Some(e) = r.pentanomial.and_then(|p| from_pentanomial(p, confidence)) {
        let mut reading = Reading {
            elo: e.elo,
            lower: e.lower,
            upper: e.upper,
            los: e.los,
            confidence: confidence.label(),
            games_for_5: games_for(&e, 5.0),
            games_to_resolve: None,
        };
        if !reading.conclusive() && e.elo.is_finite() && e.elo.abs() >= 1.0 {
            reading.games_to_resolve = games_for(&e, e.elo.abs());
        }
        return reading;
    }
    let (lower, upper) = if r.elo.is_finite() {
        (r.elo - r.elo_error, r.elo + r.elo_error)
    } else {
        (r.elo, r.elo)
    };
    Reading {
        elo: r.elo,
        lower,
        upper,
        los: r.los,
        confidence: "95%",
        games_for_5: None,
        games_to_resolve: None,
    }
}

/// What the interval says, in words.
pub fn verdict(r: &Reading) -> String {
    let c = r.confidence;
    if r.elo == f64::INFINITY {
        "the candidate won every game".into()
    } else if r.elo == f64::NEG_INFINITY {
        "the candidate lost every game".into()
    } else if r.lower > 0.0 {
        format!("the candidate is stronger (the {c} interval is above zero)")
    } else if r.upper < 0.0 {
        format!("the candidate is weaker (the {c} interval is below zero)")
    } else {
        format!("no significant difference (the {c} interval includes zero)")
    }
}

/// One line for the job log.
pub fn headline(r: &MatchResult, s: &Settings, meta: &Meta) -> String {
    let reading = read(r, s.confidence);
    format!(
        "{} vs {}: {} Elo ({} interval {}), LOS {}, {} games ({} W / {} L / {} D): {}",
        meta.candidate.label,
        meta.baseline.label,
        fmt_signed(reading.elo),
        reading.confidence,
        fmt_interval(&reading),
        fmt_los(reading.los),
        r.games,
        r.wins,
        r.losses,
        r.draws,
        verdict(&reading)
    )
}

/// The Markdown report for the job summary and the artifact.
pub fn render(r: &MatchResult, s: &Settings, meta: &Meta) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "### ⚔️ Match: `{}` vs `{}`",
        meta.candidate.label, meta.baseline.label
    );
    let _ = writeln!(out);
    if !meta.note.is_empty() {
        let _ = writeln!(out, "> {}", meta.note);
        let _ = writeln!(out);
    }
    let reading = read(r, s.confidence);
    let marker = if reading.elo > 0.0 && reading.lower > 0.0 {
        "🟢"
    } else if reading.elo < 0.0 && reading.upper < 0.0 {
        "🔴"
    } else {
        "⚪"
    };
    let _ = writeln!(out, "{marker} **{}**", verdict(&reading));
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Elo (candidate − baseline) | {} interval | LOS | Games | W / L / D | Score |",
        reading.confidence
    );
    let _ = writeln!(out, "|--:|:-:|--:|--:|:-:|--:|");
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} / {} / {} | {:.1}% |",
        fmt_signed(reading.elo),
        fmt_interval(&reading),
        fmt_los(reading.los),
        r.games,
        r.wins,
        r.losses,
        r.draws,
        r.score() * 100.0
    );
    let _ = writeln!(out);
    if let Some(n) = reading.games_to_resolve {
        let _ = writeln!(
            out,
            "Not conclusive at {}. If the true difference is the {} measured, about **{} games** would settle it at this confidence.",
            reading.confidence,
            fmt_signed(reading.elo),
            fmt_int(n)
        );
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "| | Ref | Commit | Bench signature (its own default depth) |"
    );
    let _ = writeln!(out, "|:--|:--|:--|--:|");
    for (role, side) in [("Candidate", &meta.candidate), ("Baseline", &meta.baseline)] {
        let _ = writeln!(
            out,
            "| {role} | `{}` | `{}` | {} |",
            side.label,
            &side.sha[..side.sha.len().min(7)],
            side.signature.map_or("—".to_string(), fmt_int)
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "<details><summary>Settings</summary>");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Limit: {}", s.limit_label());
    let _ = writeln!(
        out,
        "- {} games: {} openings drawn at random from `{}` ({} in the file), each played from both sides",
        s.games,
        s.games / 2,
        s.openings,
        meta.openings
    );
    let _ = writeln!(
        out,
        "- Hash {} MB per engine, {} games at once, on {}; fastchess {}",
        s.hash_mb, s.concurrency, meta.cpu, meta.fastchess
    );
    if let Some(p) = r.pentanomial {
        let _ = writeln!(
            out,
            "- Pentanomial (pairs: LL, LD, WL+DD, WD, WW): {} {} {} {} {}",
            p[0], p[1], p[2], p[3], p[4]
        );
    }
    let _ = writeln!(
        out,
        "- Elo is the candidate's over the baseline. The {} interval is computed from the pentanomial counts (pairs of games from one opening, colours swapped), so it accounts for the pairing; LOS is the likelihood the candidate is stronger",
        reading.confidence
    );
    if let Some(n) = reading.games_for_5 {
        let _ = writeln!(
            out,
            "- At this match's variance, ±5 Elo at {} would take about {} games",
            reading.confidence,
            fmt_int(n)
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "</details>");
    out
}

fn fmt_signed(v: f64) -> String {
    if v == f64::INFINITY {
        "+∞".into()
    } else if v == f64::NEG_INFINITY {
        "−∞".into()
    } else if v >= 0.0 {
        format!("+{v:.1}")
    } else {
        format!("−{:.1}", v.abs())
    }
}

fn fmt_interval(r: &Reading) -> String {
    format!("{} to {}", fmt_signed(r.lower), fmt_signed(r.upper))
}

fn fmt_los(los: f64) -> String {
    if los.is_nan() {
        "n/a".into()
    } else {
        format!("{los:.1}%")
    }
}

fn fmt_int(v: u64) -> String {
    let digits = v.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}
