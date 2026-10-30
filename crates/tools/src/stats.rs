//! Interval estimates for matches played in pairs, from pentanomial counts.
//!
//! Every opening is played twice with colours swapped, so the unit of
//! evidence is the pair, not the game: its score is 0, ¼, ½, ¾ or 1, and the
//! five counts of those (the pentanomial) give the mean and the variance per
//! pair. That variance is smaller than the per-game one because the opening's
//! bias cancels within a pair, which is why a paired match needs fewer games
//! for the same precision. The interval on the mean score is normal, at the
//! chosen two-sided confidence, and is mapped to Elo through the logistic
//! curve, so it is not symmetric around the estimate far from 50%.

/// The two-sided confidence of an interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// 95%: one result in twenty claims a difference that is not there.
    P95,
    /// 99%: one in a hundred.
    P99,
    /// 99.9%: one in a thousand.
    P999,
}

impl Confidence {
    /// Parse `95`, `99` or `99.9` (a trailing `%` is allowed).
    pub fn parse(s: &str) -> Result<Confidence, String> {
        match s.trim().trim_end_matches('%') {
            "95" => Ok(Confidence::P95),
            "99" => Ok(Confidence::P99),
            "99.9" => Ok(Confidence::P999),
            other => Err(format!("confidence must be 95, 99 or 99.9, got `{other}`")),
        }
    }

    /// The standard normal quantile for this two-sided level.
    pub fn z(self) -> f64 {
        match self {
            Confidence::P95 => 1.959_964,
            Confidence::P99 => 2.575_829,
            Confidence::P999 => 3.290_527,
        }
    }

    /// As people read it: `99%`.
    pub fn label(self) -> &'static str {
        match self {
            Confidence::P95 => "95%",
            Confidence::P99 => "99%",
            Confidence::P999 => "99.9%",
        }
    }
}

/// What a set of pairs says about the first engine against the second.
#[derive(Clone, Debug, PartialEq)]
pub struct Estimate {
    /// Elo difference at the mean score; infinite when one side scored all.
    pub elo: f64,
    /// The interval's ends in Elo; infinite when they reach 0% or 100%.
    pub lower: f64,
    /// Upper end.
    pub upper: f64,
    /// Likelihood of superiority, percent: the chance the first engine is
    /// the stronger, under the same normal model.
    pub los: f64,
    /// Pairs played.
    pub pairs: u64,
    /// Mean score per pair, `0.0..=1.0`.
    pub mean: f64,
    /// Standard deviation of one pair's score.
    pub sd_pair: f64,
    /// The confidence the interval was computed at.
    pub confidence: Confidence,
}

/// The estimate from pentanomial counts (pairs scoring 0, ¼, ½, ¾ and 1, from
/// the first engine's side). `None` for fewer than two pairs.
pub fn from_pentanomial(p: [u32; 5], confidence: Confidence) -> Option<Estimate> {
    let n: u64 = p.iter().map(|&c| c as u64).sum();
    if n < 2 {
        return None;
    }
    let score = |i: usize| i as f64 / 4.0;
    let mean = (0..5).map(|i| p[i] as f64 * score(i)).sum::<f64>() / n as f64;
    let var = (0..5)
        .map(|i| p[i] as f64 * (score(i) - mean).powi(2))
        .sum::<f64>()
        / n as f64;
    let sd_pair = var.sqrt();
    let se = sd_pair / (n as f64).sqrt();
    let z = confidence.z();
    let los = if se > 0.0 {
        100.0 * normal_cdf((mean - 0.5) / se)
    } else if mean > 0.5 {
        100.0
    } else if mean < 0.5 {
        0.0
    } else {
        50.0
    };
    Some(Estimate {
        elo: elo_of(mean),
        lower: elo_of(mean - z * se),
        upper: elo_of(mean + z * se),
        los,
        pairs: n,
        mean,
        sd_pair,
        confidence,
    })
}

/// The Elo difference that scores `score` (`0.0..=1.0`) on the logistic curve.
pub fn elo_of(score: f64) -> f64 {
    if score <= 0.0 {
        f64::NEG_INFINITY
    } else if score >= 1.0 {
        f64::INFINITY
    } else {
        -400.0 * (1.0 / score - 1.0).log10()
    }
}

/// Games (two per pair) for the interval to be at most `half_width` Elo
/// either side, at this estimate's variance per pair and confidence. Near
/// equal strength one point of mean score is about 695 Elo, and the width
/// shrinks with the square root of the pairs. `None` when the variance is
/// zero or `half_width` is not positive.
pub fn games_for(e: &Estimate, half_width: f64) -> Option<u64> {
    if e.sd_pair <= 0.0 || half_width <= 0.0 || !half_width.is_finite() {
        return None;
    }
    let m = e.mean.clamp(0.01, 0.99);
    let slope = 400.0 / (std::f64::consts::LN_10 * m * (1.0 - m));
    let pairs = (e.confidence.z() * e.sd_pair * slope / half_width).powi(2);
    Some(2 * pairs.ceil() as u64)
}

/// The standard normal cumulative distribution.
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / std::f64::consts::SQRT_2))
}

/// The error function, Abramowitz and Stegun 7.1.26 (absolute error below
/// 1.5e-7), plenty for a likelihood shown to one decimal.
fn erf(x: f64) -> f64 {
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let poly = t
        * (0.254_829_592
            + t * (-0.284_496_736
                + t * (1.421_413_741 + t * (-1.453_152_027 + t * 1.061_405_429))));
    sign * (1.0 - poly * (-x * x).exp())
}
