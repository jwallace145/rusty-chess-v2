//! Paired speed ratios, bench output and cachegrind parsing.

use rusty_chess_tools::speed::{
    BenchRun, paired_ratio, parse_bench_output, parse_bench_run, parse_cachegrind_summary,
};

#[test]
fn bench_output_yields_nodes_nps_and_time() {
    let text = "Position 1/50: ...\n===========================\nPositions       : 50\nTotal time (ms) : 821\nNodes searched  : 8986067\nNodes/second    : 10945270\n";
    assert_eq!(parse_bench_output(text), Some((8_986_067, 10_945_270)));
    assert_eq!(
        parse_bench_run(text),
        Some(BenchRun {
            nodes: 8_986_067,
            nps: 10_945_270,
            ms: 821
        })
    );
    assert_eq!(parse_bench_output("nothing"), None);
    assert_eq!(
        parse_bench_run("Nodes searched  : 1\nNodes/second    : 1\n"),
        None,
        "the time line is required"
    );
}

#[test]
fn cachegrind_summary_is_the_total_instruction_count() {
    let text = "desc: I1 cache: ...\ncmd: rusty-chess bench --depth 6\nevents: Ir\nfl=???\nfn=main\n1 5\nsummary: 12345678901\n";
    assert_eq!(parse_cachegrind_summary(text), Some(12_345_678_901));
    assert_eq!(parse_cachegrind_summary("events: Ir\n"), None);
}

#[test]
fn a_faster_machine_cancels_out_of_the_paired_ratio() {
    // The PR is 10% faster in every pair, on a machine whose speed drifts 2x over the run.
    let base = [1_000, 1_500, 2_000, 1_200, 1_800];
    let pr: Vec<u64> = base.iter().map(|b| b * 11 / 10).collect();
    let (ratio, low, high) = paired_ratio(&base, &pr).unwrap();
    assert!((ratio - 1.1).abs() < 1e-9, "{ratio}");
    assert!(
        (high - low).abs() < 1e-9,
        "no spread when every pair agrees: {low}..{high}"
    );
}

#[test]
fn noisy_pairs_widen_the_interval_around_the_mean() {
    let base = [1_000, 1_000, 1_000, 1_000, 1_000];
    let pr = [1_020, 980, 1_050, 1_000, 990];
    let (ratio, low, high) = paired_ratio(&base, &pr).unwrap();
    assert!((ratio - 1.008).abs() < 1e-9, "{ratio}");
    assert!(
        low < 1.0 && high > 1.0,
        "a +0.8% mean within noise is not a speedup: {low}..{high}"
    );
    assert!(paired_ratio(&[1, 2], &[1]).is_none());
    assert!(paired_ratio(&[0], &[1]).is_none());
}
