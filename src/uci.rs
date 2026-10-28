//! The UCI loop.
//!
//! Commands are read on the calling thread; a `go` runs the search on a
//! worker thread so that `stop` and `quit` can interrupt it through the
//! shared stop flag. `info` lines and `bestmove` are written by the worker.

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rusty_chess_core::position::Position;
use rusty_chess_core::types::Color;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{DEFAULT_HASH_MB, Limits, Searcher, mate_in};
use rusty_chess_search::telemetry::IterRecord;

/// Read commands until `quit` or end of input, writing replies to `out`.
/// Returns the writer when done.
pub fn run<R, W>(input: R, out: W) -> io::Result<W>
where
    R: BufRead,
    W: Write + Send + 'static,
{
    let out = Arc::new(Mutex::new(out));
    let mut engine = Engine::new(out.clone());
    for line in input.lines() {
        let line = line?;
        let mut words = line.split_whitespace();
        match words.next() {
            Some("uci") => {
                let mut o = out.lock().unwrap();
                writeln!(o, "id name rusty-chess {}", env!("CARGO_PKG_VERSION"))?;
                writeln!(o, "id author {}", env!("CARGO_PKG_AUTHORS"))?;
                writeln!(
                    o,
                    "option name Hash type spin default {DEFAULT_HASH_MB} min 1 max 4096"
                )?;
                writeln!(
                    o,
                    "option name Move Overhead type spin default {DEFAULT_MOVE_OVERHEAD_MS} min 0 max 5000"
                )?;
                writeln!(o, "uciok")?;
            }
            Some("isready") => writeln!(out.lock().unwrap(), "readyok")?,
            Some("ucinewgame") => {
                engine.stop();
                engine.board = Board::from_position(Position::startpos());
                engine.searcher_mut().clear_hash();
            }
            Some("setoption") => {
                engine.stop();
                engine.set_option(words)?;
            }
            Some("position") => {
                engine.stop();
                engine.set_position(words)?;
            }
            Some("go") => {
                engine.stop();
                engine.go(words);
            }
            Some("stop") => engine.stop(),
            Some("d") => {
                engine.stop();
                writeln!(out.lock().unwrap(), "{}", engine.board.position())?;
            }
            Some("quit") => break,
            _ => {}
        }
        out.lock().unwrap().flush()?;
    }
    // Let a bounded search finish and print its bestmove; cut an infinite one.
    engine.finish();
    drop(engine);
    Ok(Arc::try_unwrap(out)
        .ok()
        .expect("the search thread has been joined")
        .into_inner()
        .unwrap())
}

struct Engine<W: Write + Send + 'static> {
    board: Board,
    out: Arc<Mutex<W>>,
    stop_flag: Arc<AtomicBool>,
    /// The searcher owns the transposition table, which persists across moves
    /// of a game; it moves into the worker thread for each search and back.
    searcher: Option<Searcher>,
    worker: Option<JoinHandle<Searcher>>,
    infinite: bool,
    /// Time kept back per move for the GUI and the pipe (`Move Overhead`).
    move_overhead: Duration,
}

/// The default `Move Overhead`, in milliseconds. Enough for a local GUI;
/// a bot over the network sets more.
const DEFAULT_MOVE_OVERHEAD_MS: u64 = 10;

impl<W: Write + Send + 'static> Engine<W> {
    fn new(out: Arc<Mutex<W>>) -> Self {
        Engine {
            board: Board::from_position(Position::startpos()),
            out,
            stop_flag: Arc::new(AtomicBool::new(false)),
            searcher: Some(Searcher::new(Arc::new(AtomicBool::new(false)))),
            worker: None,
            infinite: false,
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
        }
    }

    /// Stop a running search and wait for its `bestmove`.
    fn stop(&mut self) {
        if let Some(handle) = self.worker.take() {
            self.stop_flag.store(true, Ordering::Relaxed);
            if let Ok(searcher) = handle.join() {
                self.searcher = Some(searcher);
            }
            self.stop_flag.store(false, Ordering::Relaxed);
        }
    }

    /// Wait for a running search; stop it first only if it would never end.
    fn finish(&mut self) {
        if self.infinite {
            self.stop();
        } else if let Some(handle) = self.worker.take() {
            if let Ok(searcher) = handle.join() {
                self.searcher = Some(searcher);
            }
        }
    }

    /// The searcher, with any running search stopped first.
    fn searcher_mut(&mut self) -> &mut Searcher {
        self.stop();
        self.searcher
            .get_or_insert_with(|| Searcher::new(Arc::new(AtomicBool::new(false))))
    }

    fn set_option<'a>(&mut self, words: impl Iterator<Item = &'a str>) -> io::Result<()> {
        let words: Vec<&str> = words.collect();
        // `setoption name <name> value <value>`
        let name = words.iter().position(|w| *w == "name").map(|i| {
            words[i + 1..]
                .iter()
                .take_while(|w| **w != "value")
                .cloned()
                .collect::<Vec<_>>()
                .join(" ")
        });
        let value = words
            .iter()
            .position(|w| *w == "value")
            .and_then(|i| words.get(i + 1))
            .copied();
        match (name.as_deref(), value) {
            (Some("Hash"), Some(v)) => match v.parse::<usize>() {
                Ok(mb) if (1..=4096).contains(&mb) => self.searcher_mut().set_hash_mb(mb),
                _ => writeln!(
                    self.out.lock().unwrap(),
                    "info string Hash must be 1..4096 MB, got {v}"
                )?,
            },
            (Some("Move Overhead"), Some(v)) => match v.parse::<u64>() {
                Ok(ms) if ms <= 5000 => self.move_overhead = Duration::from_millis(ms),
                _ => writeln!(
                    self.out.lock().unwrap(),
                    "info string Move Overhead must be 0..5000 ms, got {v}"
                )?,
            },
            (Some(other), _) => writeln!(
                self.out.lock().unwrap(),
                "info string unknown option {other}"
            )?,
            _ => {}
        }
        Ok(())
    }

    fn set_position<'a>(&mut self, mut words: impl Iterator<Item = &'a str>) -> io::Result<()> {
        let position = match words.next() {
            Some("startpos") => Position::startpos(),
            Some("fen") => {
                let fen: Vec<&str> = words.by_ref().take_while(|w| *w != "moves").collect();
                match Position::from_fen(&fen.join(" ")) {
                    Ok(p) => p,
                    Err(e) => {
                        writeln!(self.out.lock().unwrap(), "info string {e}")?;
                        return Ok(());
                    }
                }
            }
            _ => return Ok(()),
        };
        self.board = Board::from_position(position);
        for word in words {
            if word == "moves" {
                continue;
            }
            match self.board.position().parse_uci_move(word) {
                Some(m) => {
                    self.board.make_move(m);
                }
                None => {
                    writeln!(self.out.lock().unwrap(), "info string illegal move {word}")?;
                    break;
                }
            }
        }
        Ok(())
    }

    fn go<'a>(&mut self, mut words: impl Iterator<Item = &'a str>) {
        let mut limits = Limits::default();
        let us = self.board.position().side_to_move();
        let (mut wtime, mut btime, mut winc, mut binc) = (None, None, None, None);
        while let Some(word) = words.next() {
            let num = |words: &mut dyn Iterator<Item = &'a str>| {
                words.next().and_then(|v| v.parse::<u64>().ok())
            };
            match word {
                "depth" => limits.depth = num(&mut words).map(|d| d as u32),
                "nodes" => limits.nodes = num(&mut words),
                "movetime" => limits.movetime = num(&mut words).map(Duration::from_millis),
                "wtime" => wtime = num(&mut words).map(Duration::from_millis),
                "btime" => btime = num(&mut words).map(Duration::from_millis),
                "winc" => winc = num(&mut words).map(Duration::from_millis),
                "binc" => binc = num(&mut words).map(Duration::from_millis),
                "movestogo" => limits.movestogo = num(&mut words).map(|m| m as u32),
                "infinite" => limits.infinite = true,
                _ => {}
            }
        }
        limits.time = if us == Color::White { wtime } else { btime };
        limits.inc = if us == Color::White { winc } else { binc };
        limits.overhead = Some(self.move_overhead);
        self.infinite = limits.infinite
            || (limits.depth.is_none()
                && limits.nodes.is_none()
                && limits.movetime.is_none()
                && limits.time.is_none());

        let mut board = self.board.clone();
        let out = self.out.clone();
        let stop = self.stop_flag.clone();
        let mut searcher = self
            .searcher
            .take()
            .unwrap_or_else(|| Searcher::new(stop.clone()));
        searcher.set_stop_flag(stop);
        self.worker = Some(std::thread::spawn(move || {
            let result = searcher.search(&mut board, &limits, &mut |it: &IterRecord| {
                let _ = writeln!(out.lock().unwrap(), "{}", info_line(it));
            });
            let _ = writeln!(out.lock().unwrap(), "bestmove {}", result.best);
            let _ = out.lock().unwrap().flush();
            searcher
        }));
    }
}

/// `info depth D score (cp X | mate N) nodes N nps N time T pv ...`.
fn info_line(it: &IterRecord) -> String {
    let score = match mate_in(it.score_cp) {
        Some(n) => format!("mate {n}"),
        None => format!("cp {}", it.score_cp),
    };
    let nodes = it.nodes + it.qnodes;
    let time = (nodes * 1000).checked_div(it.nps).unwrap_or(0);
    format!(
        "info depth {} score {score} nodes {nodes} nps {} time {time} pv {}",
        it.depth, it.nps, it.pv
    )
}

impl<W: Write + Send + 'static> Drop for Engine<W> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drive(input: &str) -> String {
        let out = run(input.as_bytes(), Vec::new()).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn handshake() {
        let out = drive("uci\n");
        assert!(out.starts_with("id name rusty-chess "));
        assert!(out.ends_with("uciok\n"));
    }

    #[test]
    fn move_overhead_is_advertised_and_validated() {
        let out = drive("uci\n");
        assert!(
            out.contains("option name Move Overhead type spin default 10 min 0 max 5000"),
            "{out}"
        );
        assert_eq!(
            drive("setoption name Move Overhead value 250\nisready\n"),
            "readyok\n"
        );
        let out = drive("setoption name Move Overhead value 9000\nisready\n");
        assert!(out.contains("Move Overhead must be 0..5000 ms"), "{out}");
    }

    #[test]
    fn quit_stops_reading() {
        assert_eq!(drive("isready\nquit\nisready\n"), "readyok\n");
    }

    #[test]
    fn leading_whitespace_and_unknown_commands() {
        assert_eq!(
            drive("  isready  \nsetoption name Hash value 1\n"),
            "readyok\n"
        );
    }

    #[test]
    fn a_bad_fen_or_illegal_move_is_reported_not_fatal() {
        let out = drive("position fen nonsense\nposition startpos moves e2e5\nisready\n");
        assert!(out.contains("info string invalid fen"), "{out}");
        assert!(out.contains("info string illegal move e2e5"), "{out}");
        assert!(out.ends_with("readyok\n"));
    }

    #[test]
    fn info_lines_report_mates_as_mate_n() {
        let it = IterRecord {
            v: 1,
            fen: String::new(),
            depth: 3,
            nodes: 10,
            qnodes: 5,
            nps: 1500,
            ebf: None,
            fh_first: None,
            tt_hit: None,
            score_cp: rusty_chess_search::search::MATE - 3,
            pv: "d5f6 g7f6 c4f7".into(),
        };
        assert_eq!(
            info_line(&it),
            "info depth 3 score mate 2 nodes 15 nps 1500 time 10 pv d5f6 g7f6 c4f7"
        );
    }
}
