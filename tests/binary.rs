//! Smoke tests that drive the `rusty-chess` binary as a UCI GUI would.

use std::io::Write;
use std::process::{Command, Stdio};

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_rusty-chess"))
}

fn run_uci(input: &str) -> String {
    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn rusty-chess");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "exit status {:?}", out.status);
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn uci_handshake() {
    let out = run_uci("uci\nisready\nquit\n");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(
        lines[0],
        format!("id name rusty-chess {}", env!("CARGO_PKG_VERSION"))
    );
    assert!(lines[1].starts_with("id author "));
    assert!(lines.contains(&"uciok"));
    assert!(lines.contains(&"readyok"));
    assert_eq!(lines.last(), Some(&"readyok"));
}

#[test]
fn unknown_commands_are_ignored_and_eof_quits() {
    let out = run_uci("hello there\nisready\n");
    assert_eq!(out.trim(), "readyok");
}

#[test]
fn version_flag() {
    let out = bin().arg("--version").output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        format!("rusty-chess {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn help_lists_the_subcommands() {
    let out = bin().arg("--help").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("bench"));
    assert!(text.contains("perft"));
}

#[test]
fn bad_arguments_fail_with_a_usage_error() {
    let out = bin().args(["bench", "--depth"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--depth"));
    let out = bin().arg("frobnicate").output().unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[test]
fn bench_prints_the_signature_and_writes_jsonl() {
    let dir = std::env::temp_dir().join(format!("rusty-chess-bench-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("bench.jsonl");
    let out = bin()
        .args(["bench", "--depth", "1", "--hash", "8", "--out"])
        .arg(&out_path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Nodes searched  : "), "{text}");
    assert!(text.contains("Nodes/second    : "), "{text}");
    assert!(text.contains("Positions       : 50"), "{text}");

    let records = rusty_chess_search::telemetry::read_records(std::io::BufReader::new(
        std::fs::File::open(&out_path).unwrap(),
    ))
    .unwrap();
    // One summary per position, then the bench record last.
    assert_eq!(records.len(), 51);
    match records.last().unwrap() {
        rusty_chess_search::telemetry::Record::Bench(b) => {
            assert_eq!(b.v, 1);
            assert_eq!(b.positions, 50);
            assert_eq!(b.depth, 1);
            assert_eq!(b.hash_mb, 8);
            assert_eq!(b.stats.is_some(), cfg!(feature = "stats"));
            assert!(text.contains(&format!("Nodes searched  : {}", b.nodes)));
        }
        other => panic!("last record should be the bench record, got {other:?}"),
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn perft_divides_a_position() {
    let out = bin().args(["perft", "--depth", "3"]).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("e2e4: 600"), "{text}");
    assert!(text.contains("Nodes searched  : 8902"), "{text}");
    let out = bin()
        .args(["perft", "--fen", "not a fen"])
        .output()
        .unwrap();
    assert!(!out.status.success());
}

#[test]
fn perft_runs_the_suite_and_writes_jsonl() {
    let dir = std::env::temp_dir().join(format!("rusty-chess-perft-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out_path = dir.join("perft.jsonl");
    let suite = dir.join("suite.epd");
    std::fs::write(&suite, "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 ;D1 20 ;D2 400 ;D3 8902\n8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 3 ;D2 7\n").unwrap();
    let out = bin()
        .args(["perft", "--suite"])
        .arg(&suite)
        .args(["--max-depth", "2", "--out"])
        .arg(&out_path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Positions       : 2")
            && text.contains("Checks          : 4 passed, 0 failed"),
        "{text}"
    );
    let records = rusty_chess_search::telemetry::read_records(std::io::BufReader::new(
        std::fs::File::open(&out_path).unwrap(),
    ))
    .unwrap();
    match &records[0] {
        rusty_chess_search::telemetry::Record::Perft(p) => {
            assert_eq!((p.positions, p.max_depth, p.failures), (2, 2, 0));
            assert_eq!(p.nodes, 20 + 400 + 3 + 7);
        }
        other => panic!("{other:?}"),
    }
    // A wrong count is a failure with exit code 3.
    std::fs::write(&suite, "8/8/8/8/8/K7/P7/k7 w - - 0 1 ;D1 4\n").unwrap();
    let out = bin()
        .args(["perft", "--suite"])
        .arg(&suite)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(3));
    assert!(String::from_utf8_lossy(&out.stdout).contains("expected 4, got 3"));
    std::fs::remove_dir_all(&dir).unwrap();
}
