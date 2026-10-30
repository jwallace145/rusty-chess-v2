//! Self-play training data for NNUE.
//!
//! Workers play complete games with the engine's own search at a fixed node
//! budget, from openings of a few random plies, and keep the quiet positions
//! with their search score and the game's result. The layout of the output,
//! the record formats and the settings are specified in `docs/datagen.md`;
//! the settings live in `datagen.toml` at the repository root.
//!
//! Every shard is three files, written whole and renamed into place, the
//! manifest last, so a reader (or the uploader) only ever sees complete
//! shards: `<name>.positions.txt` (one training record per line),
//! `<name>.games.jsonl` (every game in full, so positions can be re-labelled
//! or re-filtered later without replaying) and `<name>.json` (the manifest).

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use rusty_chess_core::r#move::Move;
use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::{DEFAULT_NET_GENERATION, Network, default_network};
use rusty_chess_search::search::{Limits, Searcher, is_mate_score};
use serde::{Deserialize, Serialize};

use crate::openings::Rng;

/// The schema of the records and manifests this module writes.
pub const SCHEMA: &str = "rusty-chess-selfplay/1";

/// Generation settings, from `datagen.toml`.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Which net generation plays: 0 is the hand-written PeSTO evaluation,
    /// n the engine with net n.
    pub generation: u32,
    /// Nodes per move for the search that picks the move and scores it.
    pub nodes: u64,
    /// Random plies before the search takes over: this many or two more, at
    /// random. Always even, so both colours make the same number of random
    /// moves (an odd count gave white an extra one and black a 1.4-point
    /// edge in generation 0's results).
    pub random_plies: u32,
    /// An opening is replayed when the search scores it beyond this, in
    /// centipawns: lopsided starts teach little.
    pub max_opening_cp: i32,
    /// Transposition table per worker, MB.
    pub hash_mb: usize,
    /// Positions per shard (whole games, so a shard can run slightly over).
    pub shard_positions: usize,
    /// Win adjudication: this many consecutive plies at or beyond
    /// `win_cp` for the same side end the game as a win.
    pub win_plies: u32,
    /// Win adjudication threshold, centipawns.
    pub win_cp: i32,
    /// Draw adjudication: from this game ply on, ...
    pub draw_after_ply: u32,
    /// ... this many consecutive plies within `draw_cp` of zero end the game
    /// as a draw.
    pub draw_plies: u32,
    /// Draw adjudication threshold, centipawns.
    pub draw_cp: i32,
    /// A game that reaches this many plies is a draw.
    pub max_plies: u32,
}

impl Settings {
    /// Parse and check `datagen.toml`.
    pub fn parse(text: &str) -> Result<Settings, String> {
        #[derive(Deserialize)]
        struct File {
            datagen: Settings,
        }
        let s = toml::from_str::<File>(text)
            .map_err(|e| e.to_string())?
            .datagen;
        let checks = [
            (s.nodes > 0, "nodes must be positive"),
            (s.hash_mb > 0, "hash_mb must be positive"),
            (s.shard_positions > 0, "shard_positions must be positive"),
            (
                s.win_plies > 0 && s.draw_plies > 0,
                "adjudication plies must be positive",
            ),
            (
                s.generation == 0 || s.generation == DEFAULT_NET_GENERATION,
                "generation must be 0 (PeSTO) or the embedded net's generation",
            ),
            (
                s.random_plies % 2 == 0,
                "random_plies must be even, so both colours make as many random moves",
            ),
            (
                s.max_plies > s.random_plies + 2 && s.max_plies < 2000,
                "max_plies must exceed the random plies and stay under 2000",
            ),
        ];
        match checks.iter().find(|(ok, _)| !ok) {
            Some((_, why)) => Err(why.to_string()),
            None => Ok(s),
        }
    }

    /// The network this generation plays with: none (PeSTO) for generation
    /// 0, the embedded net for its generation. `parse` refuses any other;
    /// settings read back from an older dataset's manifest can name one, and
    /// then nothing in this build can replay it.
    pub fn network(&self) -> Option<Arc<Network>> {
        match self.generation {
            0 => None,
            g if g == DEFAULT_NET_GENERATION => Some(default_network()),
            g => panic!(
                "generation {g} was played with a net this build doesn't embed (it has generation {DEFAULT_NET_GENERATION}'s)"
            ),
        }
    }
}

/// How a game ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Termination {
    /// Checkmate on the board.
    Checkmate,
    /// Stalemate.
    Stalemate,
    /// A position repeated.
    Repetition,
    /// Fifty moves without a capture or pawn move.
    FiftyMoves,
    /// Neither side can mate.
    InsufficientMaterial,
    /// The search held a winning score long enough.
    WinAdjudication,
    /// The search held a level score long enough.
    DrawAdjudication,
    /// The ply limit.
    MaxPlies,
}

/// One position kept for training.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// The position.
    pub fen: String,
    /// The search score from white's point of view, centipawns.
    pub score_white: i32,
}

/// One finished game.
#[derive(Clone, Debug, Serialize)]
pub struct Game {
    /// The position the engine started from, after the random plies.
    pub start_fen: String,
    /// The engine's moves from there, in UCI notation.
    pub moves: Vec<String>,
    /// The search score of each move, from the side to move's point of view
    /// (mate scores as the engine reports them).
    pub scores: Vec<i32>,
    /// From white's point of view: 1.0, 0.5 or 0.0.
    pub result: f32,
    /// How it ended.
    pub termination: Termination,
    /// Positions kept for training from this game.
    #[serde(skip)]
    pub samples: Vec<Sample>,
}

/// Whether neither side has mating material: kings alone, or a single minor
/// piece on the board.
pub fn insufficient_material(pos: &Position) -> bool {
    let heavy =
        pos.by_kind(PieceKind::Pawn) | pos.by_kind(PieceKind::Rook) | pos.by_kind(PieceKind::Queen);
    let minors = pos.by_kind(PieceKind::Knight) | pos.by_kind(PieceKind::Bishop);
    heavy.is_empty() && minors.count() <= 1
}

/// Whether a searched position is worth training on: not in check, a quiet
/// best move (a capture's score is about the exchange, not the position), and
/// no mate score.
pub fn keep(pos: &Position, best: Move, score: i32) -> bool {
    !pos.in_check() && !best.is_capture() && !best.is_promotion() && !is_mate_score(score)
}

/// Play random legal plies from the start until a usable opening comes up:
/// not over, not in check, and level enough by a quick search.
fn opening(rng: &mut Rng, searcher: &mut Searcher, s: &Settings) -> Position {
    loop {
        let mut pos = Position::startpos();
        let plies = s.random_plies + 2 * (rng.next_u64() % 2) as u32;
        let mut ok = true;
        for _ in 0..plies {
            let mut list = MoveList::new();
            pos.legal_moves(&mut list);
            if list.is_empty() {
                ok = false;
                break;
            }
            let m = list.as_slice()[(rng.next_u64() % list.len() as u64) as usize];
            pos.make_move(m);
        }
        if !ok || pos.in_check() {
            continue;
        }
        let mut list = MoveList::new();
        pos.legal_moves(&mut list);
        if list.is_empty() {
            continue;
        }
        let mut board = Board::from_position(pos.clone());
        board.set_network(s.network());
        let limits = Limits {
            nodes: Some(s.nodes),
            ..Limits::default()
        };
        let r = searcher.search(&mut board, &limits, &mut |_| {});
        if r.score.abs() <= s.max_opening_cp {
            return pos;
        }
    }
}

/// How the board ends a game before the next search, if it does: no legal
/// move (mate or stalemate), a repetition (one earlier occurrence, which the
/// engine's own search scores as a draw), fifty moves, insufficient
/// material, or the ply limit (`plies` engine moves played). The result is
/// from white's point of view.
pub fn board_termination(board: &Board, plies: u32, s: &Settings) -> Option<(f32, Termination)> {
    let pos = board.position();
    let mut list = MoveList::new();
    pos.legal_moves(&mut list);
    if list.is_empty() {
        let winner_white = pos.side_to_move() == Color::Black;
        return Some(if pos.in_check() {
            (if winner_white { 1.0 } else { 0.0 }, Termination::Checkmate)
        } else {
            (0.5, Termination::Stalemate)
        });
    }
    if plies > 0 && board.is_repetition() {
        return Some((0.5, Termination::Repetition));
    }
    if board.is_fifty_moves() {
        return Some((0.5, Termination::FiftyMoves));
    }
    if insufficient_material(pos) {
        return Some((0.5, Termination::InsufficientMaterial));
    }
    if plies >= s.max_plies {
        return Some((0.5, Termination::MaxPlies));
    }
    None
}

/// Adjudication on the search's own scores, fed one search at a time.
#[derive(Clone, Debug, Default)]
pub struct Adjudicator {
    /// Consecutive plies at or beyond `win_cp` for `leader`.
    win_run: u32,
    leader: Option<Color>,
    /// Consecutive plies within `draw_cp` of zero from `draw_after_ply` on.
    draw_run: u32,
}

impl Adjudicator {
    /// Record the score (side to move's view, as the search reports it) of
    /// the search by `stm` that made engine move number `plies` (1 for the
    /// first), and say whether the game is decided. A mate score counts as
    /// a win.
    pub fn update(
        &mut self,
        stm: Color,
        score: i32,
        plies: u32,
        s: &Settings,
    ) -> Option<(f32, Termination)> {
        if score.abs() >= s.win_cp {
            let leader = if score > 0 { stm } else { !stm };
            self.win_run = if self.leader == Some(leader) {
                self.win_run + 1
            } else {
                1
            };
            self.leader = Some(leader);
            if self.win_run >= s.win_plies {
                let white = if leader == Color::White { 1.0 } else { 0.0 };
                return Some((white, Termination::WinAdjudication));
            }
        } else {
            self.win_run = 0;
            self.leader = None;
        }
        if plies >= s.draw_after_ply && score.abs() <= s.draw_cp {
            self.draw_run += 1;
            if self.draw_run >= s.draw_plies {
                return Some((0.5, Termination::DrawAdjudication));
            }
        } else {
            self.draw_run = 0;
        }
        None
    }
}

/// Play one game. Deterministic for a given `rng` state and settings.
pub fn play_game(rng: &mut Rng, searcher: &mut Searcher, s: &Settings) -> Game {
    searcher.clear_hash();
    let start = opening(rng, searcher, s);
    let start_fen = start.to_fen();
    let mut board = Board::from_position(start);
    board.set_network(s.network());
    let limits = Limits {
        nodes: Some(s.nodes),
        ..Limits::default()
    };
    let mut moves = Vec::new();
    let mut scores = Vec::new();
    let mut kept: Vec<(String, i32)> = Vec::new();
    let mut adjudicator = Adjudicator::default();
    let (result, termination) = loop {
        if let Some(end) = board_termination(&board, moves.len() as u32, s) {
            break end;
        }
        let pos = board.position().clone();
        let r = searcher.search(&mut board, &limits, &mut |_| {});
        let stm = pos.side_to_move();
        let white_score = if stm == Color::White {
            r.score
        } else {
            -r.score
        };
        if keep(&pos, r.best, r.score) {
            kept.push((pos.to_fen(), white_score));
        }
        moves.push(r.best.to_string());
        scores.push(r.score);
        if let Some(end) = adjudicator.update(stm, r.score, moves.len() as u32, s) {
            break end;
        }
        board.make_move(r.best);
    };
    Game {
        start_fen,
        moves,
        scores,
        result,
        termination,
        samples: kept
            .into_iter()
            .map(|(fen, score_white)| Sample { fen, score_white })
            .collect(),
    }
}

/// One training record: `<fen> | <score> | <result>`, the score in
/// centipawns and the result 1.0, 0.5 or 0.0, both from white's point of view.
pub fn record_line(sample: &Sample, result: f32) -> String {
    format!("{} | {} | {:.1}", sample.fen, sample.score_white, result)
}

/// Parse a training record back (for tests and for tools that read shards).
pub fn parse_record(line: &str) -> Option<(String, i32, f32)> {
    let mut parts = line.split(" | ");
    let fen = parts.next()?.to_string();
    let score = parts.next()?.parse().ok()?;
    let result = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((fen, score, result))
}

/// Where the run's identity comes from: the task and the engine that played.
#[derive(Clone, Debug, Serialize)]
pub struct Provenance {
    /// The run this shard belongs to.
    pub run: String,
    /// The task (one instance) within the run.
    pub task: String,
    /// The engine's version.
    pub engine_version: String,
    /// The commit the engine was built from.
    pub engine_sha: String,
    /// Its bench signature.
    pub engine_bench: Option<u64>,
}

/// A shard's manifest: everything needed to select it for training without
/// opening it.
#[derive(Clone, Debug, Serialize)]
pub struct Manifest {
    /// [`SCHEMA`].
    pub schema: &'static str,
    /// The shard's name, shared by its three files.
    pub shard: String,
    /// The worker and its sequence number within the task.
    pub worker: u32,
    /// Sequence number of this shard for the worker.
    pub seq: u32,
    /// The seed the worker started from.
    pub seed: u64,
    /// Who made it.
    pub provenance: Provenance,
    /// With what settings.
    pub settings: Settings,
    /// Games in the shard.
    pub games: u64,
    /// Training positions in the shard.
    pub positions: u64,
    /// Results from white's point of view: wins, draws, losses.
    pub results: [u64; 3],
    /// Games by how they ended.
    pub terminations: std::collections::BTreeMap<String, u64>,
    /// Wall-clock seconds the shard took.
    pub seconds: f64,
    /// The record files, relative to the manifest.
    pub files: [String; 2],
    /// SHA-256 of each record file's uncompressed contents, in `files`
    /// order, as hex. Filled in by [`write_shard`].
    pub sha256: [String; 2],
}

/// The shard file name: `<task>-w<worker>-s<seq>`.
pub fn shard_name(task: &str, worker: u32, seq: u32) -> String {
    format!("{task}-w{worker:03}-s{seq:05}")
}

/// Write a shard's three files into `dir`, each whole and renamed into place,
/// the manifest last, with the SHA-256 of each record file in it.
pub fn write_shard(dir: &Path, games: &[Game], manifest: &Manifest) -> std::io::Result<()> {
    let write = |name: &str, body: &[u8]| -> std::io::Result<()> {
        let tmp: PathBuf = dir.join(format!(".{name}.part"));
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(body)?;
        f.sync_all()?;
        std::fs::rename(&tmp, dir.join(name))
    };
    let mut positions = String::new();
    let mut records = String::new();
    for g in games {
        for s in &g.samples {
            let _ = writeln!(positions, "{}", record_line(s, g.result));
        }
        let _ = writeln!(
            records,
            "{}",
            serde_json::to_string(g).expect("a game serializes")
        );
    }
    let hex = |bytes: &[u8]| {
        use sha2::Digest;
        sha2::Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    let mut manifest = manifest.clone();
    manifest.sha256 = [hex(positions.as_bytes()), hex(records.as_bytes())];
    write(&manifest.files[0], positions.as_bytes())?;
    write(&manifest.files[1], records.as_bytes())?;
    write(
        &format!("{}.json", manifest.shard),
        serde_json::to_string_pretty(&manifest)
            .expect("a manifest serializes")
            .as_bytes(),
    )
}

/// What a whole run should do.
pub struct Run {
    /// Settings from `datagen.toml`.
    pub settings: Settings,
    /// Who is running it.
    pub provenance: Provenance,
    /// Worker threads, one search each.
    pub threads: u32,
    /// The run's seed; each worker derives its own.
    pub seed: u64,
    /// Where shards are written.
    pub out: PathBuf,
    /// Stop after about this many positions in all (0 for no limit).
    pub max_positions: u64,
    /// Stop after this long.
    pub max_time: Option<Duration>,
    /// Stop when this file appears (the task's graceful stop).
    pub stop_file: Option<PathBuf>,
}

/// Totals for the run.
#[derive(Debug, Default)]
pub struct Totals {
    /// Games played.
    pub games: u64,
    /// Positions kept.
    pub positions: u64,
    /// Shards written.
    pub shards: u64,
}

/// Run the workers until a limit is reached, printing progress to stderr
/// every `report` interval. Workers finish their current game and write
/// their last, partial shard before returning.
pub fn run(r: &Run, report: Duration) -> std::io::Result<Totals> {
    rusty_chess_core::attacks::init();
    std::fs::create_dir_all(&r.out)?;
    let started = Instant::now();
    let stop = AtomicBool::new(false);
    let games = AtomicU64::new(0);
    let positions = AtomicU64::new(0);
    let shards = AtomicU64::new(0);
    let should_stop = || {
        stop.load(Ordering::Relaxed)
            || (r.max_positions > 0 && positions.load(Ordering::Relaxed) >= r.max_positions)
            || r.max_time.is_some_and(|t| started.elapsed() >= t)
            || r.stop_file.as_ref().is_some_and(|f| f.exists())
    };
    let result: std::io::Result<()> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for worker in 0..r.threads {
            let (games, positions, shards, stop) = (&games, &positions, &shards, &stop);
            let should_stop = &should_stop;
            handles.push(scope.spawn(move || -> std::io::Result<()> {
                let seed = r.seed ^ (u64::from(worker) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                let mut rng = Rng::seeded(seed);
                let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
                searcher.set_hash_mb(r.settings.hash_mb);
                let mut seq = 0u32;
                loop {
                    let shard_started = Instant::now();
                    let mut batch: Vec<Game> = Vec::new();
                    let mut in_shard = 0usize;
                    while in_shard < r.settings.shard_positions && !should_stop() {
                        let g = play_game(&mut rng, &mut searcher, &r.settings);
                        in_shard += g.samples.len();
                        games.fetch_add(1, Ordering::Relaxed);
                        positions.fetch_add(g.samples.len() as u64, Ordering::Relaxed);
                        batch.push(g);
                    }
                    if !batch.is_empty() {
                        let name = shard_name(&r.provenance.task, worker, seq);
                        let mut results = [0u64; 3];
                        let mut terminations = std::collections::BTreeMap::new();
                        for g in &batch {
                            results[if g.result == 1.0 {
                                0
                            } else if g.result == 0.5 {
                                1
                            } else {
                                2
                            }] += 1;
                            let key = serde_json::to_value(g.termination)
                                .ok()
                                .and_then(|v| v.as_str().map(String::from))
                                .unwrap_or_default();
                            *terminations.entry(key).or_insert(0) += 1;
                        }
                        let manifest = Manifest {
                            schema: SCHEMA,
                            shard: name.clone(),
                            worker,
                            seq,
                            seed,
                            provenance: r.provenance.clone(),
                            settings: r.settings.clone(),
                            games: batch.len() as u64,
                            positions: in_shard as u64,
                            results,
                            terminations,
                            seconds: shard_started.elapsed().as_secs_f64(),
                            files: [
                                format!("{name}.positions.txt"),
                                format!("{name}.games.jsonl"),
                            ],
                            sha256: Default::default(),
                        };
                        if let Err(e) = write_shard(&r.out, &batch, &manifest) {
                            // A full disk stops every worker, not just this one.
                            stop.store(true, Ordering::Relaxed);
                            return Err(e);
                        }
                        shards.fetch_add(1, Ordering::Relaxed);
                        seq += 1;
                    }
                    if should_stop() {
                        return Ok(());
                    }
                }
            }));
        }
        // Progress, until every worker has returned.
        let mut last = Instant::now();
        while handles.iter().any(|h| !h.is_finished()) {
            std::thread::sleep(Duration::from_millis(200));
            if last.elapsed() >= report {
                last = Instant::now();
                let secs = started.elapsed().as_secs_f64().max(1e-9);
                let p = positions.load(Ordering::Relaxed);
                eprintln!(
                    "[{:.0}s] {} games, {} positions ({:.0}/s), {} shards",
                    secs,
                    games.load(Ordering::Relaxed),
                    p,
                    p as f64 / secs,
                    shards.load(Ordering::Relaxed)
                );
            }
        }
        for h in handles {
            h.join().expect("a worker panicked")?;
        }
        Ok(())
    });
    result?;
    Ok(Totals {
        games: games.load(Ordering::Relaxed),
        positions: positions.load(Ordering::Relaxed),
        shards: shards.load(Ordering::Relaxed),
    })
}
