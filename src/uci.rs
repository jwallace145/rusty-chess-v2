//! The UCI loop.
//!
//! Commands are read on the calling thread; a `go` runs the search on a
//! worker thread so that `stop` and `quit` can interrupt it through the
//! shared stop flag. `info` lines and `bestmove` are written by the worker.
//!
//! Pondering: `go ponder` searches the position after the move the engine
//! expects (the GUI has already played it) on the opponent's time, with the
//! clock off. `ponderhit` (the opponent played it) starts the clock with the
//! limits the `go ponder` carried, and the search carries on; `stop` (they
//! played something else) ends it. Until one of the two arrives, no
//! `bestmove` is sent, even if the search has finished. Every `bestmove`
//! names the expected reply (`bestmove e2e4 ponder e7e5`) when the principal
//! variation has one.

use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use rusty_chess_core::position::Position;
use rusty_chess_core::types::Color;
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::{Network, default_network};
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
                writeln!(o, "option name Ponder type check default false")?;
                writeln!(o, "option name EvalFile type string default <default>")?;
                writeln!(o, "uciok")?;
            }
            Some("isready") => writeln!(out.lock().unwrap(), "readyok")?,
            Some("ucinewgame") => {
                engine.stop();
                engine.board = Board::from_position(Position::startpos());
                engine.board.set_network(engine.net.clone());
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
            Some("ponderhit") => engine.ponderhit(),
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
    /// The running search's ponder flag, true until `ponderhit` (see the
    /// module documentation).
    ponder: Option<Arc<AtomicBool>>,
    /// Whether the pondering search becomes bounded after `ponderhit`.
    bounded_after_hit: bool,
    /// Time kept back per move for the GUI and the pipe (`Move Overhead`).
    move_overhead: Duration,
    /// The NNUE network: the embedded one, or `EvalFile`'s; PeSTO when `None`.
    net: Option<Arc<Network>>,
}

/// The default `Move Overhead`, in milliseconds. Enough for a local GUI;
/// a bot over the network sets more.
const DEFAULT_MOVE_OVERHEAD_MS: u64 = 10;

impl<W: Write + Send + 'static> Engine<W> {
    fn new(out: Arc<Mutex<W>>) -> Self {
        let net = Some(default_network());
        let mut board = Board::from_position(Position::startpos());
        board.set_network(net.clone());
        Engine {
            board,
            out,
            stop_flag: Arc::new(AtomicBool::new(false)),
            searcher: Some(Searcher::new(Arc::new(AtomicBool::new(false)))),
            worker: None,
            infinite: false,
            ponder: None,
            bounded_after_hit: false,
            move_overhead: Duration::from_millis(DEFAULT_MOVE_OVERHEAD_MS),
            net,
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
        self.ponder = None;
    }

    /// The opponent played the expected move: start the pondering search's
    /// clock and let it finish as an ordinary search.
    fn ponderhit(&mut self) {
        if let Some(flag) = self.ponder.take() {
            flag.store(false, Ordering::Relaxed);
            self.infinite = !self.bounded_after_hit;
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
            // A network file; `<default>` (or empty) for the embedded net,
            // `pesto` for the hand-written evaluation.
            (Some("EvalFile"), _) => {
                let path = value.unwrap_or("<default>");
                if matches!(path, "<default>" | "<empty>" | "") {
                    self.net = Some(default_network());
                    self.board.set_network(self.net.clone());
                } else if path.eq_ignore_ascii_case("pesto") {
                    self.net = None;
                    self.board.set_network(None);
                } else {
                    match std::fs::read(path)
                        .map_err(|e| e.to_string())
                        .and_then(|b| Network::from_bytes(&b))
                    {
                        Ok(net) => {
                            self.net = Some(Arc::new(net));
                            self.board.set_network(self.net.clone());
                            writeln!(
                                self.out.lock().unwrap(),
                                "info string EvalFile {path} loaded"
                            )?;
                        }
                        Err(e) => {
                            writeln!(self.out.lock().unwrap(), "info string EvalFile {path}: {e}")?
                        }
                    }
                }
            }
            // The GUI says whether it will send `go ponder`; nothing to set.
            (Some("Ponder"), Some("true" | "false")) => {}
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
        self.board.set_network(self.net.clone());
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
        let mut ponder = false;
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
                "ponder" => ponder = true,
                _ => {}
            }
        }
        limits.time = if us == Color::White { wtime } else { btime };
        limits.inc = if us == Color::White { winc } else { binc };
        limits.overhead = Some(self.move_overhead);
        let unbounded = limits.infinite
            || (limits.depth.is_none()
                && limits.nodes.is_none()
                && limits.movetime.is_none()
                && limits.time.is_none());
        // A pondering search never ends by itself until `ponderhit`.
        self.infinite = unbounded || ponder;
        self.bounded_after_hit = !unbounded;
        let ponder_flag = ponder.then(|| Arc::new(AtomicBool::new(true)));
        limits.ponder = ponder_flag.clone();
        self.ponder = ponder_flag.clone();

        let mut board = self.board.clone();
        let out = self.out.clone();
        let stop = self.stop_flag.clone();
        let mut searcher = self
            .searcher
            .take()
            .unwrap_or_else(|| Searcher::new(stop.clone()));
        searcher.set_stop_flag(stop.clone());
        self.worker = Some(std::thread::spawn(move || {
            let result = searcher.search(&mut board, &limits, &mut |it: &IterRecord| {
                let _ = writeln!(out.lock().unwrap(), "{}", info_line(it));
            });
            // UCI: while pondering, no `bestmove` until `ponderhit` or `stop`,
            // even when the search has run out of depth.
            if let Some(flag) = &ponder_flag {
                while flag.load(Ordering::Relaxed) && !stop.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
            let line = match result.pv.split_whitespace().nth(1) {
                Some(reply) => format!("bestmove {} ponder {reply}", result.best),
                None => format!("bestmove {}", result.best),
            };
            let _ = writeln!(out.lock().unwrap(), "{line}");
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

    fn bestmoves(out: &str) -> Vec<&str> {
        out.lines().filter(|l| l.starts_with("bestmove")).collect()
    }

    #[test]
    fn ponder_is_advertised_and_accepted() {
        let out = drive("uci\nsetoption name Ponder value true\nisready\n");
        assert!(
            out.contains("option name Ponder type check default false"),
            "{out}"
        );
        assert!(!out.contains("unknown option"), "{out}");
    }

    #[test]
    fn a_pondering_search_waits_for_ponderhit_or_stop_before_its_bestmove() {
        // Depth 1 finishes at once, but the bestmove must wait: here for the
        // stop at end of input, after `isready` has been answered.
        let out = drive("position startpos\ngo ponder depth 1\nisready\n");
        let ready = out.find("readyok").expect("readyok");
        let best = out.find("bestmove").expect("bestmove");
        assert!(ready < best, "bestmove came before the stop: {out}");
        assert_eq!(bestmoves(&out).len(), 1, "{out}");
    }

    #[test]
    fn ponderhit_turns_the_search_into_a_timed_one_that_finishes_by_itself() {
        let out = drive("position startpos\ngo ponder movetime 50\nponderhit\n");
        let moves = bestmoves(&out);
        assert_eq!(moves.len(), 1, "{out}");
        // A 50 ms search of the start position has a reply to ponder on.
        let words: Vec<&str> = moves[0].split_whitespace().collect();
        assert_eq!(words.len(), 4, "{out}");
        assert_eq!(words[2], "ponder", "{out}");
    }

    #[test]
    fn stop_ends_a_pondering_search_with_one_bestmove() {
        let out = drive("position startpos\ngo ponder wtime 60000 btime 60000\nstop\nisready\n");
        assert_eq!(bestmoves(&out).len(), 1, "{out}");
    }

    #[test]
    fn eval_file_loads_a_network_and_reports_a_bad_one() {
        use rusty_chess_search::nnue::FILE_BYTES;
        let dir = std::env::temp_dir().join(format!("uci-evalfile-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // An all-zero network of the right size: legal, if not very good.
        let good = dir.join("zero.nnue");
        std::fs::write(&good, vec![0u8; FILE_BYTES]).unwrap();
        let bad = dir.join("short.nnue");
        std::fs::write(&bad, vec![0u8; 10]).unwrap();
        let out = drive(&format!(
            "uci\nsetoption name EvalFile value {}\nsetoption name EvalFile value {}\nposition startpos\ngo depth 3\n",
            bad.display(),
            good.display()
        ));
        assert!(
            out.contains("option name EvalFile type string default <default>"),
            "{out}"
        );
        assert!(out.contains("short.nnue: a 768"), "{out}");
        assert!(out.contains("zero.nnue loaded"), "{out}");
        assert_eq!(bestmoves(&out).len(), 1, "{out}");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_ordinary_search_names_its_expected_reply() {
        let out = drive("position startpos\ngo depth 4\n");
        let moves = bestmoves(&out);
        assert_eq!(moves.len(), 1, "{out}");
        assert!(moves[0].contains(" ponder "), "{out}");
    }
}
