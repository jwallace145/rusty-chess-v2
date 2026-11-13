//! Research-only runtime parameters (the overnight lab build, never merged).
//!
//! Every parameter defaults to the value `main` has compiled in, so a lab
//! binary with nothing set searches exactly as `main` does (the bench
//! signature is the check). A parameter is set by `setoption name L_<NAME>
//! value <int>` or by an environment variable `LAB_<NAME>=<int>` read once at
//! start-up (for `bench`).

use std::sync::atomic::{AtomicI32, Ordering};

macro_rules! params {
    ($($name:ident = $default:expr),* $(,)?) => {
        $(pub static $name: AtomicI32 = AtomicI32::new($default);)*
        /// Every parameter: its name, its cell and its default.
        pub static ALL: &[(&str, &AtomicI32, i32)] = &[$((stringify!($name), &$name, $default)),*];
    };
}

params! {
    // Null-move pruning: R = NMP_BASE + depth / NMP_DIV + min((eval - beta) / NMP_EVAL_DIV, NMP_EVAL_MAX).
    NMP_BASE = 2,
    NMP_DIV = 4,
    NMP_EVAL_DIV = 0,
    NMP_EVAL_MAX = 3,
    NMP_MIN_DEPTH = 3,
    // Fractional form for tuning (used when NMP_A64 > 0):
    // R = (NMP_A64 + NMP_B64 * depth) / 64 + the evaluation term.
    NMP_A64 = 0,
    NMP_B64 = 21,
    // Reverse futility pruning.
    RFP_DEPTH = 6,
    RFP_MARGIN = 80,
    RFP_IMPROVING = 0,
    // Futility pruning of quiets.
    FP_DEPTH = 6,
    FP_BASE = 100,
    FP_MARGIN = 100,
    // Late move pruning: quiets from index LMP_BASE + depth^2, halved when
    // LMP_IMPROVING is set and the node is not improving.
    LMP_DEPTH = 4,
    LMP_BASE = 3,
    LMP_IMPROVING = 0,
    // The late-move-pruning threshold as a percentage of LMP_BASE + depth^2.
    LMP_PCT = 100,
    // Late move reductions.
    LMR_HIST_DIV = 512,
    LMR_PV = 0,
    LMR_CUT = 1,
    LMR_IMPROVING = 0,
    // Razoring (0 = off): depth <= RAZOR_DEPTH and eval + RAZOR_MARGIN * depth < alpha.
    RAZOR_DEPTH = 0,
    RAZOR_MARGIN = 250,
    // Quiescence futility (0 = off): skip a capture when stand pat + margin + victim <= alpha.
    QS_FUT = 0,
    // The victim's value in that rule: 0 = the classical SEE values, 1 = the
    // net's measured values (300, 775, 880, 1100, 1680).
    QS_FUT_NET = 0,
    // Singular extensions.
    SE_MIN_DEPTH = 8,
    SE_MARGIN = 2,
    SE_DOUBLE = 0,
    SE_DOUBLE_MAX = 6,
    SE_NEG = 0,
    // Transposition table.
    TT_NO_PV_CUT = 0,
    // 1 = the table score replaces the evaluation in either direction, 2 =
    // only where it lowers it (prune less), 3 = only where it raises it.
    TT_EVAL = 0,
    // Aspiration window half-width.
    ASP_DELTA = 35,
    // History bonus: 0 = min(depth^2, 1200); 1 = min(HIST_A * depth + HIST_B, HIST_MAX).
    HIST_MODE = 0,
    HIST_A = 150,
    HIST_B = -100,
    HIST_MAX = 1800,
    // History pruning (0 = off): skip a quiet at depth <= HP_DEPTH whose history < -HP_MARGIN * depth.
    HP_DEPTH = 0,
    HP_MARGIN = 400,
    // SEE pruning of quiets.
    SEE_DEPTH = 8,
    SEE_QUIET = 64,
    // SEE pruning of captures (0 = off): skip a capture at depth <= SEE_DEPTH losing more than SEE_CAP * depth.
    SEE_CAP = 0,
    // Internal iterative reductions.
    IIR_DEPTH = 4,
    // The LMR table, in hundredths: LMR_BASE / 100 + ln(depth) * ln(index) / (LMR_DIV / 100).
    LMR_BASE = 75,
    LMR_DIV = 225,
    // Capture history (0 = off): added to a capture's ordering score divided by this.
    CAPHIST = 0,
    // Continuation history four plies back (0 = off).
    CONT4 = 0,
    // ProbCut (0 = off): at depth >= PC_DEPTH, a capture whose search at
    // depth - 4 beats beta + PC_MARGIN ends the node.
    PC_MARGIN = 0,
    PC_DEPTH = 5,
    // ProbCut's capture filter: 0 = any capture that does not lose material;
    // n = the exchange must win (beta + margin - eval) / n in classical units
    // (3 converts the net's scale to the classical one).
    PC_SEE_DIV = 0,
    // Futility pruning of captures in the main search (0 = off): at depth <=
    // CAPFUT_DEPTH, skip a capture when eval + CAPFUT_BASE + CAPFUT * depth +
    // the victim's net-scale value <= alpha.
    CAPFUT = 0,
    CAPFUT_BASE = 300,
    CAPFUT_DEPTH = 6,
    // Late move reduction of losing captures, in plies (0 = off).
    LMR_NOISY = 0,
}

/// The value of `p`.
#[inline(always)]
pub fn get(p: &AtomicI32) -> i32 {
    p.load(Ordering::Relaxed)
}

/// Set the parameter called `name` (without the `L_` prefix); false if unknown.
pub fn set(name: &str, value: i32) -> bool {
    for (n, cell, _) in ALL {
        if n.eq_ignore_ascii_case(name) {
            cell.store(value, Ordering::Relaxed);
            return true;
        }
    }
    false
}

/// Read `LAB_<NAME>` environment variables (once, at start-up).
pub fn init_from_env() {
    for (n, cell, _) in ALL {
        if let Ok(v) = std::env::var(format!("LAB_{n}")) {
            if let Ok(v) = v.parse::<i32>() {
                cell.store(v, Ordering::Relaxed);
            }
        }
    }
}

/// The parameters that differ from their defaults, as `NAME=value` pairs.
pub fn describe() -> String {
    ALL.iter()
        .filter(|(_, cell, d)| cell.load(Ordering::Relaxed) != *d)
        .map(|(n, cell, _)| format!("{n}={}", cell.load(Ordering::Relaxed)))
        .collect::<Vec<_>>()
        .join(" ")
}
