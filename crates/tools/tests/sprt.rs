//! Parsing fastchess output and rendering the SPRT comment.

use rusty_chess_tools::gate::{Sprt, Targets};
use rusty_chess_tools::sprt::{
    Meta, Notes, Verdict, fastchess_args, interpret, parse, passes, render, render_dual,
    render_dual_with_notes,
};

const LOG: &str = "\
Started game 1 of 3000 (pr vs base)
Finished game 1 (pr vs base): 1-0 {White mates}
--------------------------------------------------
Results of pr vs base (200000 nodes, 1t, NULLMB, openings.epd):
Elo: 5.00 +/- 20.00, nElo: 6.00 +/- 25.00
LOS: 60.00 %, DrawRatio: 40.00 %, PairsRatio: 1.10
Games: 40, Wins: 12, Losses: 10, Draws: 18, Points: 21.0 (52.50 %)
Ptnml(0-2): [1, 4, 10, 4, 1], WL/DD Ratio: 0.50
LLR: 0.10 (3.4%) (-2.94, 2.94) [0.00, 10.00]
--------------------------------------------------
[INFO] SPRT ([0.00, 10.00]) completed - H1 was accepted
--------------------------------------------------
Results of pr vs base (200000 nodes, 1t, NULLMB, openings.epd):
Elo: 23.45 +/- 9.87, nElo: 30.12 +/- 12.34
LOS: 99.99 %, DrawRatio: 41.20 %, PairsRatio: 2.10
Games: 800, Wins: 300, Losses: 180, Draws: 320, Points: 460.0 (57.50 %)
Ptnml(0-2): [10, 60, 180, 120, 30], WL/DD Ratio: 0.85
LLR: 2.96 (100.0%) (-2.94, 2.94) [0.00, 10.00]
--------------------------------------------------
Tournament finished
";

#[test]
fn the_last_block_wins_and_the_verdict_is_read() {
    let r = parse(LOG).unwrap();
    assert_eq!((r.elo, r.elo_error), (23.45, 9.87));
    assert_eq!(r.los, 99.99);
    assert_eq!((r.games, r.wins, r.losses, r.draws), (800, 300, 180, 320));
    assert_eq!((r.llr, r.llr_lower, r.llr_upper), (2.96, -2.94, 2.94));
    assert_eq!(r.pentanomial, Some([10, 60, 180, 120, 30]));
    assert_eq!(r.verdict, Verdict::H1);
}

#[test]
fn h0_and_inconclusive_are_distinguished() {
    let h0 = LOG.replace("H1 was accepted", "H0 was accepted");
    assert_eq!(parse(&h0).unwrap().verdict, Verdict::H0);
    let capped: String = LOG
        .lines()
        .filter(|l| !l.contains("completed"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(parse(&capped).unwrap().verdict, Verdict::Inconclusive);
    assert!(parse("nothing here").is_err());
}

#[test]
fn the_comment_is_compact_and_says_what_was_played() {
    let r = parse(LOG).unwrap();
    let md = render(
        &r,
        &Sprt::default(),
        &Meta {
            head_label: "abc1234".into(),
            base_label: "main@def5678".into(),
            fastchess: "v1.8.2-alpha".into(),
            openings: 300,
        },
    );
    assert!(md.starts_with("### ⚔️ SPRT"), "{md}");
    assert!(md.contains("🟢 **H1 accepted**"), "{md}");
    assert!(
        md.contains("| +23.4 ± 9.9 | 100.0% | 800 | 300 / 180 / 320 | 2.96 (-2.94, 2.94) |"),
        "{md}"
    );
    assert!(
        md.contains("200,000 nodes per move")
            && md.contains("300 openings")
            && md.contains("v1.8.2-alpha")
    );
    assert!(md.contains("elo0 = 0, elo1 = 10"));
    assert_eq!(md.matches("<details>").count(), 1);
    assert!(md.lines().count() < 20, "{}", md.lines().count());
    let mut h0 = r.clone();
    h0.verdict = Verdict::H0;
    assert!(
        render(
            &h0,
            &Sprt::default(),
            &Meta {
                head_label: "a".into(),
                base_label: "b".into(),
                fastchess: "v".into(),
                openings: 1
            }
        )
        .contains("🔴 **H0 accepted**")
    );
}

#[test]
fn settings_parse_from_the_targets_file_and_become_fastchess_arguments() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[sprt]\nnodes_per_move = 50_000\nelo1 = 5.0\nmax_rounds = 100\n").unwrap();
    assert_eq!(t.sprt.nodes_per_move, 50_000);
    assert_eq!(t.sprt.elo1, 5.0);
    assert_eq!(t.sprt.max_rounds, 100);
    assert_eq!(t.sprt.concurrency, 3, "default kept");
    let args = fastchess_args(&t.sprt).join(" ");
    assert!(args.contains("-each tc=600+6 nodes=50000"), "{args}");
    assert!(
        args.contains("-sprt elo0=0 elo1=5 alpha=0.05 beta=0.05"),
        "{args}"
    );
    assert!(
        args.contains("-rounds 100")
            && args.contains("-games 2 -repeat")
            && args.contains("-openings file=tests/openings/openings.epd format=epd order=random")
    );
}

#[test]
fn a_timed_run_plays_at_the_configured_time_control_instead_of_nodes() {
    let mut t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[sprt]\ntc = \"5+0.05\"\n").unwrap();
    assert!(!t.sprt.timed, "timed is never read from the file");
    t.sprt.timed = true;
    let args = fastchess_args(&t.sprt).join(" ");
    assert!(args.starts_with("-each tc=5+0.05 -openings"), "{args}");
    assert!(!args.contains("nodes="), "{args}");
    let r = parse(LOG).unwrap();
    let meta = Meta {
        head_label: "pr".into(),
        base_label: "base".into(),
        fastchess: "v1".into(),
        openings: 300,
    };
    let text = render(&r, &t.sprt, &meta);
    assert!(text.contains("Timed games at 5+0.05"), "{text}");
    assert!(!text.contains("nodes per move"), "{text}");
}

#[test]
fn only_an_accepted_h1_passes_and_a_smoke_test_may_be_inconclusive() {
    assert!(passes(Verdict::H1, false));
    assert!(!passes(Verdict::H0, false));
    assert!(!passes(Verdict::Inconclusive, false));
    assert!(passes(Verdict::Inconclusive, true));
    assert!(!passes(Verdict::H0, true));
}

#[test]
fn a_balanced_result_renders_los_as_not_available() {
    let log = LOG.replace("LOS: 99.99 %", "LOS: nan %");
    let r = parse(&log).unwrap();
    assert!(r.los.is_nan());
    let md = render(
        &r,
        &Sprt::default(),
        &Meta {
            head_label: "a".into(),
            base_label: "b".into(),
            fastchess: "v".into(),
            openings: 1,
        },
    );
    assert!(md.contains("| n/a |"), "{md}");
}

#[test]
fn the_pair_of_verdicts_is_read_with_the_timed_one_as_the_gate() {
    use Verdict::{H0, H1, Inconclusive};
    assert!(interpret(Some(H1), Some(H1)).contains("**merge**"));
    assert!(interpret(Some(H1), Some(Inconclusive)).contains("`perf` pass"));
    assert!(interpret(Some(H1), Some(H0)).contains("do not merge"));
    assert!(interpret(Some(Inconclusive), Some(H1)).contains("speed gain"));
    assert!(interpret(Some(H0), Some(H0)).contains("no gain at a clock"));
    assert!(interpret(None, Some(H1)).contains("**merge**"));
    assert!(interpret(Some(H1), None).contains("did not finish"));
}

#[test]
fn a_dual_comment_shows_the_gate_and_both_runs() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[sprt]\ntc = \"5+0.05\"\n").unwrap();
    let meta = Meta {
        head_label: "abc1234".into(),
        base_label: "main@def5678".into(),
        fastchess: "v1".into(),
        openings: 300,
    };
    let h1 = parse(LOG).unwrap();
    let mut inconclusive = h1.clone();
    inconclusive.verdict = Verdict::Inconclusive;
    let md = render_dual(Some(&inconclusive), Some(&h1), &t.sprt, &meta);
    assert!(md.starts_with("### ⚔️ SPRT\n\n🔴 **Gate: fail**"), "{md}");
    assert!(md.contains("`perf` pass"), "{md}");
    assert!(md.contains("#### Timed, 5+0.05 (the gate)"), "{md}");
    assert!(
        md.contains("#### Fixed nodes, 200,000 per move (informational)"),
        "{md}"
    );
    assert!(md.contains("Timed games at 5+0.05"), "{md}");
    assert!(
        md.contains("200,000 nodes per move: whether the search"),
        "{md}"
    );
    let md = render_dual(Some(&h1), None, &t.sprt, &meta);
    assert!(
        md.contains("🟢 **Gate: pass**") && md.contains("Did not finish"),
        "{md}"
    );
    // The single-run comment keeps its heading.
    assert!(render(&h1, &t.sprt, &meta).starts_with("### ⚔️ SPRT\n\n🟢"));
}

#[test]
fn an_interrupted_leg_shows_its_note_and_never_a_verdict() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[sprt]\ntc = \"5+0.05\"\n").unwrap();
    let meta = Meta {
        head_label: "abc1234".into(),
        base_label: "main@def5678".into(),
        fastchess: "v1".into(),
        openings: 300,
    };
    let h1 = parse(LOG).unwrap();
    let notes = Notes {
        timed: Some("⚠️ **Interrupted by spot reclamation** at 12:00.".into()),
        nodes: None,
    };
    // Even with a partial H1 log, a noted timed leg is no result.
    let md = render_dual_with_notes(Some(&h1), Some(&h1), &notes, &t.sprt, &meta);
    assert!(md.contains("⚪ **Gate: no result**"), "{md}");
    assert!(
        md.contains("the infrastructure stopped the timed test"),
        "{md}"
    );
    assert!(md.contains("Interrupted by spot reclamation"), "{md}");
    assert!(!md.contains("🟢 **Gate: pass**"), "{md}");
    // Without notes it is the plain dual comment.
    assert_eq!(
        render_dual_with_notes(Some(&h1), None, &Notes::default(), &t.sprt, &meta),
        render_dual(Some(&h1), None, &t.sprt, &meta)
    );
}

#[test]
fn concurrency_can_be_raised_for_a_bigger_runner() {
    let t = Targets::parse("[movegen]\nmin_perft_nps = { local = 0, ci = 0 }\n[search]\nmin_nps = { local = 0, ci = 0 }\n[efficiency]\nmin_fail_high_first = 0.0\nmax_ebf = 0.0\n[sprt]\n").unwrap();
    let mut s = t.sprt.clone();
    s.concurrency = 60;
    let args = fastchess_args(&s).join(" ");
    assert!(args.contains("-concurrency 60"), "{args}");
}
