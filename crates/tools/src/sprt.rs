//! The SPRT report: parse fastchess's final result block and render the PR
//! comment.
//!
//! fastchess prints, per rating interval and at the end:
//!
//! ```text
//! Results of pr vs base (200000 nodes, 1t, NULLMB, openings.epd):
//! Elo: 23.45 +/- 9.87, nElo: 30.12 +/- 12.34
//! LOS: 99.99 %, DrawRatio: 41.20 %, PairsRatio: 2.10
//! Games: 800, Wins: 300, Losses: 180, Draws: 320, Points: 460.0 (57.50 %)
//! Ptnml(0-2): [10, 60, 180, 120, 30], WL/DD Ratio: 0.85
//! LLR: 2.96 (100.0%) (-2.94, 2.94) [0.00, 10.00]
//! ```
//!
//! and, when a verdict is reached, `SPRT ([0.00, 10.00]) completed - H1 was
//! accepted`. The last block in the log is the one that counts.

use std::fmt::Write as _;

use crate::gate::Sprt;

/// The outcome of the sequential test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The change is at least `elo1` better: accepted.
    H1,
    /// The change is no better than `elo0`: rejected.
    H0,
    /// The round cap was reached first.
    Inconclusive,
}

/// What fastchess reported.
#[derive(Clone, Debug, PartialEq)]
pub struct SprtResult {
    /// Elo difference of the first engine (the PR) over the second (the base).
    pub elo: f64,
    /// Its error bar.
    pub elo_error: f64,
    /// Likelihood of superiority, in percent.
    pub los: f64,
    /// Games played.
    pub games: u32,
    /// Wins, losses and draws for the PR.
    pub wins: u32,
    /// Losses.
    pub losses: u32,
    /// Draws.
    pub draws: u32,
    /// The log-likelihood ratio and its bounds.
    pub llr: f64,
    /// Lower LLR bound.
    pub llr_lower: f64,
    /// Upper LLR bound.
    pub llr_upper: f64,
    /// The pentanomial counts, if reported.
    pub pentanomial: Option<[u32; 5]>,
    /// The verdict.
    pub verdict: Verdict,
}

/// The result of a match, with or without an SPRT.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchResult {
    /// Elo difference of the first engine over the second; may be infinite
    /// when one side scored everything.
    pub elo: f64,
    /// Its 95% error bar.
    pub elo_error: f64,
    /// Likelihood of superiority, in percent (NaN when undefined).
    pub los: f64,
    /// Games played.
    pub games: u32,
    /// Wins for the first engine.
    pub wins: u32,
    /// Losses for the first engine.
    pub losses: u32,
    /// Draws.
    pub draws: u32,
    /// The pentanomial counts, if reported.
    pub pentanomial: Option<[u32; 5]>,
}

impl MatchResult {
    /// The first engine's score in `0.0..=1.0`.
    pub fn score(&self) -> f64 {
        if self.games == 0 {
            0.5
        } else {
            (self.wins as f64 + 0.5 * self.draws as f64) / self.games as f64
        }
    }
}

/// Parse the last result block of any fastchess match log.
pub fn parse_match(log: &str) -> Result<MatchResult, String> {
    parse_any(log).map(|(m, _, _)| m)
}

/// Parse the last result block in a fastchess SPRT log.
pub fn parse(log: &str) -> Result<SprtResult, String> {
    let (m, llr, verdict) = parse_any(log)?;
    let (llr, llr_lower, llr_upper) = llr.ok_or("no `LLR:` line in the log")?;
    Ok(SprtResult {
        elo: m.elo,
        elo_error: m.elo_error,
        los: m.los,
        games: m.games,
        wins: m.wins,
        losses: m.losses,
        draws: m.draws,
        llr,
        llr_lower,
        llr_upper,
        pentanomial: m.pentanomial,
        verdict,
    })
}

type Parsed = (MatchResult, Option<(f64, f64, f64)>, Verdict);

fn parse_any(log: &str) -> Result<Parsed, String> {
    let mut elo = None;
    let mut los = None;
    let mut games = None;
    let mut llr = None;
    let mut penta = None;
    let mut verdict = Verdict::Inconclusive;
    for line in log.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("Elo: ") {
            let value = rest.split(',').next().unwrap_or("");
            let mut parts = value.split("+/-").map(str::trim);
            let e = parts.next().and_then(|v| v.parse::<f64>().ok());
            let err = parts.next().and_then(|v| v.parse::<f64>().ok());
            if let (Some(e), Some(err)) = (e, err) {
                elo = Some((e, err));
            }
        } else if let Some(rest) = line.strip_prefix("LOS: ") {
            // fastchess prints `nan %` when the result is exactly balanced.
            los = Some(
                rest.split('%')
                    .next()
                    .and_then(|v| v.trim().parse::<f64>().ok())
                    .unwrap_or(f64::NAN),
            );
        } else if let Some(rest) = line.strip_prefix("Games: ") {
            // `800, Wins: 300, Losses: 180, Draws: 320, Points: ...`
            let mut parts = rest.split(',');
            let g = parts.next().and_then(|v| v.trim().parse::<u32>().ok());
            let mut nums = parts
                .filter_map(|part| part.split(':').nth(1))
                .filter_map(|v| v.trim().parse::<u32>().ok());
            if let (Some(g), Some(w), Some(l), Some(d)) = (g, nums.next(), nums.next(), nums.next())
            {
                games = Some((g, w, l, d));
            }
        } else if let Some(rest) = line.strip_prefix("Ptnml(0-2): [") {
            let nums: Vec<u32> = rest
                .split(']')
                .next()
                .unwrap_or("")
                .split(',')
                .filter_map(|v| v.trim().parse().ok())
                .collect();
            if nums.len() == 5 {
                penta = Some([nums[0], nums[1], nums[2], nums[3], nums[4]]);
            }
        } else if let Some(rest) = line.strip_prefix("LLR: ") {
            let value = rest
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok());
            let bounds = rest
                .split('(')
                .nth(2)
                .and_then(|b| b.split(')').next())
                .map(|b| {
                    let mut it = b.split(',').filter_map(|v| v.trim().parse::<f64>().ok());
                    (it.next(), it.next())
                });
            if let (Some(v), Some((Some(lo), Some(hi)))) = (value, bounds) {
                llr = Some((v, lo, hi));
            }
        } else if line.contains("SPRT (") && line.contains("completed") {
            verdict = if line.contains("H1 was accepted") {
                Verdict::H1
            } else if line.contains("H0 was accepted") {
                Verdict::H0
            } else {
                Verdict::Inconclusive
            };
        }
    }
    let (elo, elo_error) = elo.ok_or("no `Elo:` line in the log")?;
    let los = los.ok_or("no `LOS:` line in the log")?;
    let (games, wins, losses, draws) = games.ok_or("no `Games:` line in the log")?;
    Ok((
        MatchResult {
            elo,
            elo_error,
            los,
            games,
            wins,
            losses,
            draws,
            pentanomial: penta,
        },
        llr,
        verdict,
    ))
}

/// Labels for the comment.
#[derive(Clone, Debug, PartialEq)]
pub struct Meta {
    /// The PR's head, e.g. a short SHA.
    pub head_label: String,
    /// The base it played against, e.g. `main@abc1234`.
    pub base_label: String,
    /// The fastchess version used.
    pub fastchess: String,
    /// How many openings were available.
    pub openings: usize,
}

/// Render the sticky comment.
pub fn render(result: &SprtResult, settings: &Sprt, meta: &Meta) -> String {
    let (glyph, word) = match result.verdict {
        Verdict::H1 => ("🟢", "**H1 accepted**: the change gains Elo"),
        Verdict::H0 => ("🔴", "**H0 accepted**: the change does not gain Elo"),
        Verdict::Inconclusive => (
            "⚪",
            "**inconclusive** at the round cap; more games or a smaller `elo1` needed",
        ),
    };
    let mut out = String::new();
    let _ = writeln!(out, "### ⚔️ SPRT");
    let _ = writeln!(out);
    let _ = writeln!(out, "{glyph} {word}");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Elo | LOS | Games | W / L / D | LLR |");
    let _ = writeln!(out, "|--:|--:|--:|:-:|--:|");
    let los = if result.los.is_nan() {
        "n/a".to_string()
    } else {
        format!("{:.1}%", result.los)
    };
    let _ = writeln!(
        out,
        "| {:+.1} ± {:.1} | {los} | {} | {} / {} / {} | {:.2} ({:.2}, {:.2}) |",
        result.elo,
        result.elo_error,
        result.games,
        result.wins,
        result.losses,
        result.draws,
        result.llr,
        result.llr_lower,
        result.llr_upper
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "<details><summary>Details</summary>");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- PR `{}` vs base `{}`, both built on the same runner with the same flags",
        meta.head_label, meta.base_label
    );
    if settings.timed {
        let _ = writeln!(
            out,
            "- Timed games at {} on the runner, so the verdict includes time management and the runner's speed (for a change that fixed nodes cannot see); {} openings, each played from both sides; fastchess {}",
            settings.tc, meta.openings, meta.fastchess
        );
    } else {
        let _ = writeln!(
            out,
            "- {} nodes per move, so the verdict does not depend on the runner's speed (speed is judged by the bench gate); {} openings, each played from both sides; fastchess {}",
            fmt_int(settings.nodes_per_move),
            meta.openings,
            meta.fastchess
        );
    }
    let _ = writeln!(
        out,
        "- Hypotheses: elo0 = {}, elo1 = {}, α = {}, β = {}; at most {} rounds. Settings live in `perf-targets.toml` under `[sprt]`",
        settings.elo0, settings.elo1, settings.alpha, settings.beta, settings.max_rounds
    );
    if let Some(p) = result.pentanomial {
        let _ = writeln!(
            out,
            "- Pentanomial (pairs: LL, LD, WL+DD, WD, WW): {} {} {} {} {}",
            p[0], p[1], p[2], p[3], p[4]
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "</details>");
    out
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

/// The fastchess arguments for `settings`, one per element, excluding the
/// engine commands.
pub fn fastchess_args(settings: &Sprt) -> Vec<String> {
    let s = |v: String| v;
    let each = if settings.timed {
        vec![s("-each".into()), s(format!("tc={}", settings.tc))]
    } else {
        vec![
            s("-each".into()),
            s("tc=600+6".into()),
            s(format!("nodes={}", settings.nodes_per_move)),
        ]
    };
    let mut args = each;
    args.extend(vec![
        s("-openings".into()),
        s(format!("file={}", settings.openings)),
        s("format=epd".into()),
        s("order=random".into()),
        s("-games".into()),
        s("2".into()),
        s("-repeat".into()),
        s("-rounds".into()),
        s(settings.max_rounds.to_string()),
        s("-concurrency".into()),
        s(settings.concurrency.to_string()),
        s("-maxmoves".into()),
        s("200".into()),
        s("-draw".into()),
        s("movenumber=40".into()),
        s("movecount=8".into()),
        s("score=10".into()),
        s("-resign".into()),
        s("movecount=3".into()),
        s("score=800".into()),
        s("twosided=true".into()),
        s("-sprt".into()),
        s(format!("elo0={}", settings.elo0)),
        s(format!("elo1={}", settings.elo1)),
        s(format!("alpha={}", settings.alpha)),
        s(format!("beta={}", settings.beta)),
        s("-ratinginterval".into()),
        s("20".into()),
        s("-report".into()),
        s("penta=true".into()),
        s("-recover".into()),
    ]);
    args
}
