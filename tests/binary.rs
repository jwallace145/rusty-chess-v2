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
fn go_depth_answers_with_info_lines_and_a_legal_bestmove() {
    let out = run_uci("position startpos moves e2e4 e7e5\ngo depth 3\nquit\n");
    let lines: Vec<&str> = out.lines().collect();
    let infos: Vec<&&str> = lines
        .iter()
        .filter(|l| l.starts_with("info depth "))
        .collect();
    assert_eq!(infos.len(), 3, "{out}");
    assert!(
        infos[2].contains(" depth 3 ")
            && infos[2].contains(" score cp ")
            && infos[2].contains(" nodes ")
            && infos[2].contains(" nps ")
            && infos[2].contains(" pv "),
        "{}",
        infos[2]
    );
    let best = lines.last().unwrap();
    assert!(best.starts_with("bestmove "), "{out}");
    let mv = best.split(' ').nth(1).unwrap();
    let pos = rusty_chess_core::position::Position::from_fen(
        "rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2",
    )
    .unwrap();
    assert!(pos.parse_uci_move(mv).is_some(), "{mv} is not legal");
}

#[test]
fn position_fen_and_mate_scores() {
    let out = run_uci("position fen 6k1/5ppp/8/8/8/8/8/R5K1 w - - 0 1\ngo depth 2\nquit\n");
    assert!(out.contains(" score mate 1 "), "{out}");
    assert!(out.trim_end().ends_with("bestmove a1a8"), "{out}");
}

#[test]
fn go_with_a_clock_and_go_infinite_with_stop() {
    let started = std::time::Instant::now();
    let out = run_uci("position startpos\ngo wtime 1000 btime 1000 winc 0 binc 0\nquit\n");
    assert!(out.contains("bestmove "), "{out}");
    assert!(started.elapsed() < std::time::Duration::from_secs(2));

    let mut child = bin()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    stdin
        .write_all(b"position startpos\ngo infinite\n")
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(200));
    stdin.write_all(b"stop\nquit\n").unwrap();
    let out = child.wait_with_output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success());
    assert_eq!(
        text.lines().filter(|l| l.starts_with("bestmove ")).count(),
        1,
        "{text}"
    );
}

#[test]
fn ucinewgame_and_the_d_command() {
    let out = run_uci("ucinewgame\nposition startpos moves e2e4\nd\nquit\n");
    assert!(
        out.contains("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1"),
        "{out}"
    );
}

#[test]
fn a_repetition_in_the_game_history_counts() {
    // After the shuffle the start position is on the board for the third time; the engine must know it is a draw.
    let out = run_uci(
        "position fen rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1 moves g1f3 g8f6 f3g1\ngo depth 3\nquit\n",
    );
    assert!(out.trim_end().ends_with("bestmove f6g8"), "{out}");
    assert!(out.contains(" score cp 0 "), "{out}");
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
    assert!(text.contains("bestmove "), "{text}");
    assert!(text.contains("Positions       : 50"), "{text}");

    let records = rusty_chess_search::telemetry::read_records(std::io::BufReader::new(
        std::fs::File::open(&out_path).unwrap(),
    ))
    .unwrap();
    // Iteration records and one summary per position, then the bench record last.
    assert!(records.len() > 51, "{}", records.len());
    let iters = records
        .iter()
        .filter(|r| matches!(r, rusty_chess_search::telemetry::Record::Iter(_)))
        .count();
    assert_eq!(iters, 50, "one iteration record per position at depth 1");
    match records.last().unwrap() {
        rusty_chess_search::telemetry::Record::Bench(b) => {
            assert_eq!(b.v, 1);
            assert_eq!(b.positions, 50);
            assert_eq!(b.depth, 1);
            assert_eq!(b.hash_mb, 8);
            assert_eq!(b.stats.is_some(), cfg!(feature = "stats"));
            assert!(b.nodes > 0, "the bench searches now");
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
