//! `datagen-audit`: check self-play shards before training on them.
//!
//! ```text
//! datagen-audit check DIR [--config datagen.toml] [--research N] [--bullet]
//! datagen-audit research DIR N        fresh re-search of N records: label agreement, depth
//! datagen-audit determinism DIR       regenerate a shard's first games from its seed
//! datagen-audit outcomes DIR          game result by score bucket, by how games ended
//! datagen-audit thresholds DIR        would lower win adjudication have called the right winner?
//! datagen-audit adjudication DIR N    deep search and playout of N win-adjudicated games
//! datagen-audit repetition DIR N      deep search before N games' repeating moves
//! datagen-audit openings DIR          what the start positions and early play look like
//! datagen-audit bullet DIR            every record through bulletformat's real parser
//! ```
//!
//! DIR holds downloaded shards as they are in S3, flattened: `<shard>.json`,
//! `<shard>.positions.txt.gz`, `<shard>.games.jsonl.gz` (see
//! `ci/datagen-sample.sh` and `docs/datagen-runbook.md`).
//!
//! `check` replays every game from its start position with the move
//! generator, rebuilds the training records independently of `datagen` and
//! compares them byte for byte, confirms each game's result from its final
//! position, verifies checksums and manifest counts, and gathers the quality
//! statistics. Every P0 (correctness) check must find zero problems: any
//! failure exits 1. P1 (quality) statistics are compared with the bars in
//! `[audit]` of the config and reported as PASS or WARN. `research`,
//! `determinism` and the deep checks search with the engine this binary was
//! built from; run them from a build at the data's `engine_sha` (printed) for
//! exact comparisons.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use flate2::read::GzDecoder;
use rusty_chess_core::movegen::MoveList;
use rusty_chess_core::position::Position;
use rusty_chess_core::types::{Color, PieceKind};
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::Network;
use rusty_chess_search::search::{Limits, MATE_BOUND, Searcher, is_mate_score};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// A gzip file's contents: its lines and the SHA-256 of the uncompressed bytes.
fn read_gz(p: &Path) -> (Vec<String>, String) {
    let mut raw = Vec::new();
    std::io::Read::read_to_end(
        &mut GzDecoder::new(std::fs::File::open(p).unwrap()),
        &mut raw,
    )
    .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let hash = Sha256::digest(&raw)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let lines = BufReader::new(raw.as_slice())
        .lines()
        .map(|l| l.unwrap())
        .collect();
    (lines, hash)
}

struct Shard {
    name: String,
    manifest: Value,
    positions: Vec<String>,
    games: Vec<Value>,
    /// SHA-256 of the uncompressed positions and games files.
    hashes: [String; 2],
}

fn load(dir: &Path) -> Vec<Shard> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let n = e.unwrap().file_name().to_string_lossy().to_string();
            n.strip_suffix(".json").map(String::from)
        })
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let (positions, hp) = read_gz(&dir.join(format!("{name}.positions.txt.gz")));
            let (games, hg) = read_gz(&dir.join(format!("{name}.games.jsonl.gz")));
            Shard {
                manifest: serde_json::from_str(
                    &std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap(),
                )
                .unwrap(),
                positions,
                games: games
                    .iter()
                    .map(|l| serde_json::from_str(l).unwrap())
                    .collect(),
                hashes: [hp, hg],
                name,
            }
        })
        .collect()
}

/// The network the dataset's games were played with, from the manifests'
/// generation: re-searches must evaluate as the games did.
fn dataset_net(shards: &[Shard]) -> Option<Arc<Network>> {
    let generation = &shards[0].manifest["settings"]["generation"];
    assert!(
        shards
            .iter()
            .all(|s| &s.manifest["settings"]["generation"] == generation),
        "the shards mix generations; audit one at a time"
    );
    let settings: rusty_chess_tools::datagen::Settings =
        serde_json::from_value(shards[0].manifest["settings"].clone()).unwrap();
    settings.network()
}

/// The P1 quality bars, from `[audit]` in `datagen.toml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bars {
    draw_rate: [f64; 2],
    color_gap_max: f64,
    duplicate_rate_max: f64,
    distinct_openings_min: f64,
    near_zero_share: [f64; 2],
    decisive_share_max: f64,
    endgame_share_max: f64,
    kept_share: [f64; 2],
    median_game_plies: [f64; 2],
    research_correlation_min: f64,
    research_same_sign_min: f64,
    research_median_depth_min: f64,
}

fn bars(config: &Path) -> Bars {
    #[derive(Deserialize)]
    struct File {
        audit: Bars,
    }
    let text =
        std::fs::read_to_string(config).unwrap_or_else(|e| panic!("{}: {e}", config.display()));
    toml::from_str::<File>(&text)
        .unwrap_or_else(|e| panic!("{}: {e}", config.display()))
        .audit
}

/// One line of the verdict: a P1 statistic against its bar.
fn bar(name: &str, value: f64, lo: f64, hi: f64, warns: &mut u32) {
    let ok = value >= lo && value <= hi;
    if !ok {
        *warns += 1;
    }
    let range = match (lo == f64::MIN, hi == f64::MAX) {
        (true, _) => format!("<= {hi}"),
        (_, true) => format!(">= {lo}"),
        _ => format!("{lo}..{hi}"),
    };
    println!(
        "  {} {name}: {value:.4} (bar {range})",
        if ok { "PASS" } else { "WARN" }
    );
}

fn insufficient(pos: &Position) -> bool {
    let heavy =
        pos.by_kind(PieceKind::Pawn) | pos.by_kind(PieceKind::Rook) | pos.by_kind(PieceKind::Queen);
    let minors = pos.by_kind(PieceKind::Knight) | pos.by_kind(PieceKind::Bishop);
    heavy.is_empty() && minors.count() <= 1
}

/// FEN without the move counters, for duplicate counting.
fn key4(fen: &str) -> String {
    fen.split(' ').take(4).collect::<Vec<_>>().join(" ")
}

fn pct(a: u64, b: u64) -> f64 {
    if b == 0 {
        0.0
    } else {
        100.0 * a as f64 / b as f64
    }
}

fn quantile(v: &mut [i64], q: f64) -> i64 {
    v.sort_unstable();
    if v.is_empty() {
        return 0;
    }
    v[((v.len() - 1) as f64 * q).round() as usize]
}

fn replay(dir: &Path, bc: &Bars) -> (u64, u32) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let mut c: BTreeMap<&str, u64> = BTreeMap::new();
    let inc = |c: &mut BTreeMap<&str, u64>, k: &'static str, n: u64| *c.entry(k).or_insert(0) += n;
    let mut fail: Vec<String> = Vec::new();
    // Examples of record mismatches, which are counted on their own.
    let mut examples: Vec<String> = Vec::new();
    let mut term_count: BTreeMap<String, u64> = BTreeMap::new();
    let mut term_mismatch: BTreeMap<String, u64> = BTreeMap::new();
    let mut result_by_term: BTreeMap<String, [u64; 3]> = BTreeMap::new();
    let mut game_len: Vec<i64> = Vec::new();
    let mut kept_per_game: Vec<i64> = Vec::new();
    let mut start_fens: HashSet<String> = HashSet::new();
    let mut start_scores: Vec<i64> = Vec::new();
    let mut rep_final_abs: Vec<i64> = Vec::new();
    let mut seen_pos: HashMap<String, u32> = HashMap::new();
    let mut hist: BTreeMap<i64, u64> = BTreeMap::new();
    let mut buckets = [0u64; 4];
    let mut phase = [0u64; 3];
    let mut pieces_hist: BTreeMap<u32, u64> = BTreeMap::new();
    let mut castling_any = 0u64;
    let mut imbalance = [0u64; 3];
    let mut polarity: [[u64; 3]; 2] = [[0; 3]; 2]; // [score > +300, score < -300] × [W, D, B]
    let mut stm_score: [(i64, u64); 2] = [(0, 0); 2];
    let mut long_game_share = (0u64, 0u64);
    let mut total_records = 0u64;
    let mut seeds: HashMap<u64, (String, u64)> = HashMap::new();
    let (mut checksummed, mut checksum_bad) = (0u64, 0u64);
    let mut ply_filter = [0u64; 5]; // searched, in check, capture best, promotion best, mate score
    let mut in_game_dup_runs = 0u64;

    for sh in &shards {
        let m = &sh.manifest;
        // A worker's shards share its seed; two workers must never.
        let who = (
            m["provenance"]["task"].as_str().unwrap_or("").to_string(),
            m["worker"].as_u64().unwrap(),
        );
        if let Some(prev) = seeds.insert(m["seed"].as_u64().unwrap(), who.clone()) {
            if prev != who {
                fail.push(format!("{}: seed shared with worker {:?}", sh.name, prev));
            }
        }
        // Checksums, when the manifest has them (shards written since they were added).
        if let Some(sums) = m.get("sha256").and_then(|v| v.as_array()) {
            checksummed += 1;
            if sums.len() != 2 || sums[0] != sh.hashes[0] || sums[1] != sh.hashes[1] {
                checksum_bad += 1;
                fail.push(format!("{}: checksum mismatch", sh.name));
            }
        }
        // Manifest counts against the files.
        if m["positions"].as_u64().unwrap() != sh.positions.len() as u64 {
            fail.push(format!(
                "{}: manifest positions {} vs {} lines",
                sh.name,
                m["positions"],
                sh.positions.len()
            ));
        }
        if m["games"].as_u64().unwrap() != sh.games.len() as u64 {
            fail.push(format!(
                "{}: manifest games {} vs {} lines",
                sh.name,
                m["games"],
                sh.games.len()
            ));
        }
        let mut line_idx = 0usize;
        let settings = &m["settings"];
        let (win_cp, win_plies) = (
            settings["win_cp"].as_i64().unwrap() as i32,
            settings["win_plies"].as_u64().unwrap() as usize,
        );
        let (draw_cp, draw_plies, draw_after) = (
            settings["draw_cp"].as_i64().unwrap() as i32,
            settings["draw_plies"].as_u64().unwrap() as usize,
            settings["draw_after_ply"].as_u64().unwrap() as usize,
        );
        let max_plies = settings["max_plies"].as_u64().unwrap() as usize;
        let mut mresults = [0u64; 3];
        for g in &sh.games {
            inc(&mut c, "games", 1);
            let start = g["start_fen"].as_str().unwrap();
            start_fens.insert(key4(start));
            let moves: Vec<&str> = g["moves"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let scores: Vec<i32> = g["scores"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
            let result = g["result"].as_f64().unwrap() as f32;
            let term = g["termination"].as_str().unwrap().to_string();
            mresults[if result == 1.0 {
                0
            } else if result == 0.5 {
                1
            } else {
                2
            }] += 1;
            if moves.len() != scores.len() {
                fail.push(format!(
                    "{}: moves {} vs scores {}",
                    sh.name,
                    moves.len(),
                    scores.len()
                ));
                continue;
            }
            if let Some(&s0) = scores.first() {
                start_scores.push(s0 as i64);
            }
            let mut pos = Position::from_fen(start).unwrap();
            if pos.to_fen() != start {
                fail.push(format!("start FEN does not round-trip: {start}"));
            }
            let mut history: Vec<u64> = vec![pos.key()];
            let mut expected: Vec<(String, i32)> = Vec::new();
            let mut ok = true;
            let mut prev_kept = String::new();
            for (i, uci) in moves.iter().enumerate() {
                // A searched position is never terminal.
                let mut list = MoveList::new();
                pos.legal_moves(&mut list);
                if list.is_empty() {
                    fail.push(format!(
                        "{}: a move recorded from a finished position",
                        sh.name
                    ));
                    ok = false;
                    break;
                }
                let Some(mv) = pos.parse_uci_move(uci) else {
                    fail.push(format!(
                        "{}: illegal move {uci} in {}",
                        sh.name,
                        pos.to_fen()
                    ));
                    ok = false;
                    break;
                };
                let s = scores[i];
                ply_filter[0] += 1;
                let check = pos.in_check();
                let mate = is_mate_score(s);
                if check {
                    ply_filter[1] += 1;
                }
                if mv.is_capture() {
                    ply_filter[2] += 1;
                }
                if mv.is_promotion() {
                    ply_filter[3] += 1;
                }
                if mate {
                    ply_filter[4] += 1;
                }
                if mv.is_castle() {
                    inc(&mut c, "castling moves", 1);
                }
                if mv.is_en_passant() {
                    inc(&mut c, "en passant moves", 1);
                }
                if mv.is_promotion() {
                    inc(&mut c, "promotion moves", 1);
                }
                if s.abs() >= MATE_BOUND {
                    inc(&mut c, "mate scores seen in play", 1);
                }
                if !check && !mv.is_capture() && !mv.is_promotion() && !mate {
                    let fen = pos.to_fen();
                    let white = if pos.side_to_move() == Color::White {
                        s
                    } else {
                        -s
                    };
                    // Successive kept positions with the same placement within a game.
                    if key4(&fen) == prev_kept {
                        in_game_dup_runs += 1;
                    }
                    prev_kept = key4(&fen);
                    expected.push((fen, white));
                }
                pos.make_move(mv);
                history.push(pos.key());
            }
            if !ok {
                continue;
            }
            game_len.push(moves.len() as i64);
            kept_per_game.push(expected.len() as i64);
            // The positions file holds this game's records next, in order.
            for (fen, white) in &expected {
                let want = format!("{fen} | {white} | {result:.1}");
                match sh.positions.get(line_idx) {
                    Some(line) if *line == want => {}
                    Some(line) => {
                        if examples.len() < 10 {
                            examples.push(format!(
                                "{}: record mismatch\n  want {want}\n  got  {line}",
                                sh.name
                            ));
                        }
                        inc(&mut c, "record mismatches", 1);
                    }
                    None => fail.push(format!("{}: positions file ended early", sh.name)),
                }
                line_idx += 1;
                total_records += 1;
                // FEN round-trip of every record.
                if Position::from_fen(fen).map(|p| p.to_fen()).as_deref() != Ok(fen.as_str()) {
                    inc(&mut c, "record FENs that do not round-trip", 1);
                }
            }
            // The termination, checked on the final position.
            let mut list = MoveList::new();
            pos.legal_moves(&mut list);
            let n = moves.len();
            let stm = pos.side_to_move();
            let rep = {
                let span = (pos.halfmove_clock() as usize).min(history.len() - 1);
                let k = *history.last().unwrap();
                (1..=span / 2).any(|b| history[history.len() - 1 - 2 * b] == k)
            };
            let (consistent, expect_result) = match term.as_str() {
                "checkmate" => (
                    list.is_empty() && pos.in_check(),
                    if stm == Color::White { 0.0 } else { 1.0 },
                ),
                "stalemate" => (list.is_empty() && !pos.in_check(), 0.5),
                "repetition" => (rep, 0.5),
                "fifty-moves" => (pos.halfmove_clock() >= 100, 0.5),
                "insufficient-material" => (insufficient(&pos), 0.5),
                "max-plies" => (n >= max_plies, 0.5),
                "draw-adjudication" => (
                    n >= draw_after
                        && n >= draw_plies
                        && scores[n - draw_plies..].iter().all(|s| s.abs() <= draw_cp),
                    0.5,
                ),
                "win-adjudication" => {
                    // The last win_plies scores all at least win_cp for one side.
                    let tail = &scores[n.saturating_sub(win_plies)..];
                    let mut sides = Vec::new();
                    // Ply j's mover: the side to move before move j.
                    for (k, &s) in tail.iter().enumerate() {
                        let j = n - tail.len() + k;
                        let mover_white = (j % 2 == 0)
                            == (Position::from_fen(start).unwrap().side_to_move() == Color::White);
                        sides.push(if s > 0 { mover_white } else { !mover_white });
                    }
                    let all = tail.len() == win_plies
                        && tail.iter().all(|s| s.abs() >= win_cp)
                        && sides.iter().all(|&w| w == sides[0]);
                    (
                        all,
                        if sides.first() == Some(&true) {
                            1.0
                        } else {
                            0.0
                        },
                    )
                }
                _ => (false, -1.0),
            };
            *term_count.entry(term.clone()).or_insert(0) += 1;
            if !consistent || expect_result != result {
                *term_mismatch.entry(term.clone()).or_insert(0) += 1;
                if fail.len() < 30 {
                    fail.push(format!("{}: {term} not confirmed (consistent {consistent}, result {result} vs {expect_result}), final {}", sh.name, pos.to_fen()));
                }
            }
            if term == "repetition" {
                if let Some(&s) = scores.last() {
                    rep_final_abs.push(s.abs() as i64);
                }
            }
            let r = result_by_term.entry(term).or_insert([0; 3]);
            r[if result == 1.0 {
                0
            } else if result == 0.5 {
                1
            } else {
                2
            }] += 1;
            if n > 200 {
                long_game_share.0 += expected.len() as u64;
            }
            long_game_share.1 += expected.len() as u64;

            // Statistics over the game's records.
            for (fen, white) in &expected {
                let p = Position::from_fen(fen).unwrap();
                *seen_pos.entry(key4(fen)).or_insert(0) += 1;
                let b = (*white as i64).clamp(-3000, 3000) / 100 * 100;
                *hist.entry(b).or_insert(0) += 1;
                let a = white.abs();
                buckets[if a <= 100 {
                    0
                } else if a <= 300 {
                    1
                } else if a <= 700 {
                    2
                } else {
                    3
                }] += 1;
                let pieces = p.occupied().count();
                *pieces_hist.entry(pieces).or_insert(0) += 1;
                let val = |k: PieceKind| match k {
                    PieceKind::Knight | PieceKind::Bishop => 3,
                    PieceKind::Rook => 5,
                    PieceKind::Queen => 9,
                    _ => 0,
                };
                let mut npm = 0;
                let mut mat = [0i32; 2];
                for k in [
                    PieceKind::Pawn,
                    PieceKind::Knight,
                    PieceKind::Bishop,
                    PieceKind::Rook,
                    PieceKind::Queen,
                ] {
                    for (ci, col) in [Color::White, Color::Black].iter().enumerate() {
                        let n = (p.by_kind(k) & p.by_color(*col)).count() as i32;
                        mat[ci] += n * if k == PieceKind::Pawn { 1 } else { val(k) };
                        npm += n * val(k);
                    }
                }
                // Phase by non-pawn material (both sides): 62 at the start.
                phase[if npm >= 50 {
                    0
                } else if npm > 20 {
                    1
                } else {
                    2
                }] += 1;
                let d = (mat[0] - mat[1]).abs();
                imbalance[if d == 0 {
                    0
                } else if d <= 2 {
                    1
                } else {
                    2
                }] += 1;
                if fen.split(' ').nth(2) != Some("-") {
                    castling_any += 1;
                }
                if *white > 300 {
                    polarity[0][(2.0 - 2.0 * result) as usize] += 1;
                }
                if *white < -300 {
                    polarity[1][(2.0 - 2.0 * result) as usize] += 1;
                }
                let si = if p.side_to_move() == Color::White {
                    0
                } else {
                    1
                };
                stm_score[si].0 += *white as i64;
                stm_score[si].1 += 1;
            }
        }
        if line_idx != sh.positions.len() {
            fail.push(format!(
                "{}: {} lines left over after the last game",
                sh.name,
                sh.positions.len() - line_idx
            ));
        }
        let mr = m["results"].as_array().unwrap();
        if (0..3).any(|k| mr[k].as_u64().unwrap() != mresults[k]) {
            fail.push(format!(
                "{}: manifest results {:?} vs {:?}",
                sh.name, mr, mresults
            ));
        }
    }

    let games = c["games"];
    println!(
        "== Replay of {} shards: {} games, {} records",
        shards.len(),
        games,
        total_records
    );
    println!("failures: {}", fail.len());
    for e in &examples {
        println!("  FAIL {e}");
    }
    for f in fail.iter().take(30) {
        println!("  FAIL {f}");
    }
    for (k, v) in &c {
        println!("  {k}: {v}");
    }
    println!("\n== Terminations (count, unconfirmed, W/D/B)");
    for (t, n) in &term_count {
        let r = result_by_term[t];
        println!(
            "  {t:22} {n:6} ({:.1}%)  unconfirmed {}  W/D/B {}/{}/{}",
            pct(*n, games),
            term_mismatch.get(t).copied().unwrap_or(0),
            r[0],
            r[1],
            r[2]
        );
    }
    let w: u64 = result_by_term.values().map(|r| r[0]).sum();
    let d: u64 = result_by_term.values().map(|r| r[1]).sum();
    let b: u64 = result_by_term.values().map(|r| r[2]).sum();
    println!(
        "  results: white {:.1}%  draw {:.1}%  black {:.1}%",
        pct(w, games),
        pct(d, games),
        pct(b, games)
    );
    let mut gl = game_len.clone();
    println!(
        "\n== Game length (engine plies after the opening): mean {:.1}, p10 {}, median {}, p90 {}, max {}",
        gl.iter().sum::<i64>() as f64 / gl.len() as f64,
        quantile(&mut gl, 0.1),
        quantile(&mut gl, 0.5),
        quantile(&mut gl, 0.9),
        quantile(&mut gl, 1.0)
    );
    let mut kp = kept_per_game.clone();
    println!(
        "   records per game: mean {:.1}, p10 {}, median {}, p90 {}; games > 200 plies give {:.1}% of records",
        kp.iter().sum::<i64>() as f64 / kp.len() as f64,
        quantile(&mut kp, 0.1),
        quantile(&mut kp, 0.5),
        quantile(&mut kp, 0.9),
        pct(long_game_share.0, long_game_share.1)
    );
    println!(
        "\n== Filtering, over every searched ply ({}):",
        ply_filter[0]
    );
    println!(
        "  in check {:.1}%, best move a capture {:.1}%, a promotion {:.2}%, mate score {:.2}%; kept {:.1}%",
        pct(ply_filter[1], ply_filter[0]),
        pct(ply_filter[2], ply_filter[0]),
        pct(ply_filter[3], ply_filter[0]),
        pct(ply_filter[4], ply_filter[0]),
        pct(total_records, ply_filter[0])
    );
    println!(
        "\n== Openings: {} games, {} distinct start positions ({:.2}%)",
        games,
        start_fens.len(),
        pct(start_fens.len() as u64, games)
    );
    let mut ss = start_scores.clone();
    println!(
        "   first search score: p10 {} median {} p90 {} (|score| > 200: {:.1}%)",
        quantile(&mut ss, 0.1),
        quantile(&mut ss, 0.5),
        quantile(&mut ss, 0.9),
        pct(
            start_scores.iter().filter(|s| s.abs() > 200).count() as u64,
            start_scores.len() as u64
        )
    );
    println!(
        "\n== Duplicates (placement, side, castling, en passant): {} records, {} distinct ({:.2}% duplicate); seen 2+ times: {} positions",
        total_records,
        seen_pos.len(),
        100.0 - pct(seen_pos.len() as u64, total_records),
        seen_pos.values().filter(|&&n| n > 1).count()
    );
    let top: u32 = seen_pos.values().copied().max().unwrap_or(0);
    println!(
        "   most repeated position occurs {} times; consecutive equal kept positions within a game: {}",
        top, in_game_dup_runs
    );
    println!(
        "\n== Scores (white's view, cp): |s|<=100 {:.1}%  100-300 {:.1}%  300-700 {:.1}%  >700 {:.1}%  (mate/invalid kept: {})",
        pct(buckets[0], total_records),
        pct(buckets[1], total_records),
        pct(buckets[2], total_records),
        pct(buckets[3], total_records),
        c.get("kept mate").copied().unwrap_or(0)
    );
    println!("   histogram (100 cp bins, clamped at ±3000):");
    for (k, v) in &hist {
        if *v * 1000 >= total_records {
            println!("     {k:>6} {:>6.2}%", pct(*v, total_records));
        }
    }
    println!(
        "   polarity: white score > +300 → W/D/B {:?}; < -300 → {:?}",
        polarity[0], polarity[1]
    );
    println!(
        "   mean white-view score with white to move {:.1}, black to move {:.1}",
        stm_score[0].0 as f64 / stm_score[0].1 as f64,
        stm_score[1].0 as f64 / stm_score[1].1 as f64
    );
    let mut ra = rep_final_abs.clone();
    println!(
        "   repetition games' last score |s|: median {} p90 {} max {}",
        quantile(&mut ra, 0.5),
        quantile(&mut ra, 0.9),
        quantile(&mut ra, 1.0)
    );
    println!(
        "\n== Phases (non-pawn material, both sides; 62 at the start): opening/full {:.1}%  middlegame {:.1}%  endgame (<=20) {:.1}%",
        pct(phase[0], total_records),
        pct(phase[1], total_records),
        pct(phase[2], total_records)
    );
    println!(
        "   material balance: equal {:.1}%, within 2 pawns {:.1}%, more {:.1}%; castling rights present {:.1}%",
        pct(imbalance[0], total_records),
        pct(imbalance[1], total_records),
        pct(imbalance[2], total_records),
        pct(castling_any, total_records)
    );
    print!("   pieces on board:");
    for (k, v) in &pieces_hist {
        if *v * 200 >= total_records {
            print!(" {k}:{:.1}%", pct(*v, total_records));
        }
    }
    println!();

    // The verdict.
    let p0 = fail.len() as u64
        + c.get("record mismatches").copied().unwrap_or(0)
        + c.get("record FENs that do not round-trip")
            .copied()
            .unwrap_or(0)
        + term_mismatch.values().sum::<u64>();
    println!("\n== P0 correctness (every check must find zero)");
    let line =
        |name: &str, n: u64| println!("  {} {name}: {n}", if n == 0 { "PASS" } else { "FAIL" });
    line(
        "records that differ from the replayed game",
        c.get("record mismatches").copied().unwrap_or(0),
    );
    line(
        "terminations not confirmed by the final position",
        term_mismatch.values().sum(),
    );
    line(
        "record FENs that do not round-trip",
        c.get("record FENs that do not round-trip")
            .copied()
            .unwrap_or(0),
    );
    line("checksum mismatches", checksum_bad);
    line(
        "other failures (illegal moves, counts, seeds, file shapes)",
        fail.len() as u64 - checksum_bad,
    );
    if checksummed < shards.len() as u64 {
        println!(
            "  note: {} of {} shards predate per-file checksums (gzip's CRC still checked them)",
            shards.len() as u64 - checksummed,
            shards.len()
        );
    }
    println!("\n== P1 quality bars ([audit] in the config)");
    let mut warns = 0u32;
    let g = games as f64;
    let mut gl2 = game_len.clone();
    bar(
        "draw rate",
        d as f64 / g,
        bc.draw_rate[0],
        bc.draw_rate[1],
        &mut warns,
    );
    bar(
        "white-black win gap",
        (w as f64 - b_wins(&result_by_term) as f64).abs() / g,
        f64::MIN,
        bc.color_gap_max,
        &mut warns,
    );
    bar(
        "duplicate positions",
        1.0 - seen_pos.len() as f64 / total_records as f64,
        f64::MIN,
        bc.duplicate_rate_max,
        &mut warns,
    );
    bar(
        "distinct openings",
        start_fens.len() as f64 / g,
        bc.distinct_openings_min,
        f64::MAX,
        &mut warns,
    );
    bar(
        "|score| <= 100 share",
        buckets[0] as f64 / total_records as f64,
        bc.near_zero_share[0],
        bc.near_zero_share[1],
        &mut warns,
    );
    bar(
        "|score| > 700 share",
        buckets[3] as f64 / total_records as f64,
        f64::MIN,
        bc.decisive_share_max,
        &mut warns,
    );
    bar(
        "endgame share",
        phase[2] as f64 / total_records as f64,
        f64::MIN,
        bc.endgame_share_max,
        &mut warns,
    );
    bar(
        "kept share of searched plies",
        total_records as f64 / ply_filter[0] as f64,
        bc.kept_share[0],
        bc.kept_share[1],
        &mut warns,
    );
    bar(
        "median game length (engine plies)",
        quantile(&mut gl2, 0.5) as f64,
        bc.median_game_plies[0],
        bc.median_game_plies[1],
        &mut warns,
    );
    (p0, warns)
}

fn b_wins(r: &BTreeMap<String, [u64; 3]>) -> u64 {
    r.values().map(|v| v[2]).sum()
}

/// Returns the correlation within ±1500, the same-sign share and the median depth.
fn research(dir: &Path, n: usize) -> (f64, f64, f64) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let nodes = shards[0].manifest["settings"]["nodes"].as_u64().unwrap();
    let net = dataset_net(&shards);
    let mut recs: Vec<(String, i32)> = Vec::new();
    for sh in &shards {
        for (i, l) in sh.positions.iter().enumerate() {
            if i % 97 == 0 {
                let parts: Vec<&str> = l.split(" | ").collect();
                recs.push((parts[0].to_string(), parts[1].parse().unwrap()));
            }
        }
    }
    recs.truncate(n);
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.set_hash_mb(16);
    let mut outliers: Vec<(i32, String, i32, i32, u32)> = Vec::new();
    let mut depths: Vec<i64> = Vec::new();
    let mut diffs: Vec<i64> = Vec::new();
    let (mut same_sign, mut signed, mut within50) = (0u64, 0u64, 0u64);
    let (mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0f64, 0f64, 0f64, 0f64, 0f64);
    for (fen, recorded) in &recs {
        let pos = Position::from_fen(fen).unwrap();
        s.clear_hash();
        let mut b = Board::from_position(pos.clone());
        b.set_network(net.clone());
        let r = s.search(
            &mut b,
            &Limits {
                nodes: Some(nodes),
                ..Limits::default()
            },
            &mut |_| {},
        );
        let white = if pos.side_to_move() == Color::White {
            r.score
        } else {
            -r.score
        };
        depths.push(r.depth as i64);
        diffs.push((white - recorded).abs() as i64);
        if (white - recorded).abs() <= 50 {
            within50 += 1;
        }
        if recorded.abs() >= 50 {
            signed += 1;
            if (white > 0) == (*recorded > 0) {
                same_sign += 1;
            }
        }
        outliers.push((
            (white - recorded).abs(),
            fen.clone(),
            *recorded,
            white,
            r.depth,
        ));
        let (x, y) = (*recorded as f64, white as f64);
        sx += x;
        sy += y;
        sxx += x * x;
        syy += y * y;
        sxy += x * y;
    }
    let k = recs.len() as f64;
    let corr = (k * sxy - sx * sy) / ((k * sxx - sx * sx).sqrt() * (k * syy - sy * sy).sqrt());
    let mut dd = depths.clone();
    let mut df = diffs.clone();
    println!(
        "== Fresh re-search of {} sampled records at {} nodes (empty table, no game history)",
        recs.len(),
        nodes
    );
    println!(
        "   depth reached: p10 {} median {} p90 {} min {} max {}",
        quantile(&mut dd, 0.1),
        quantile(&mut dd, 0.5),
        quantile(&mut dd, 0.9),
        quantile(&mut dd, 0.0),
        quantile(&mut dd, 1.0)
    );
    println!(
        "   score vs recorded label: correlation {:.3}; |diff| median {} p90 {}; within 50 cp {:.1}%; same sign where |label| >= 50: {:.1}%",
        corr,
        quantile(&mut df, 0.5),
        quantile(&mut df, 0.9),
        pct(within50, recs.len() as u64),
        pct(same_sign, signed)
    );
    // Correlation without the extreme tail.
    let kept: Vec<&(i32, String, i32, i32, u32)> = outliers
        .iter()
        .filter(|o| o.2.abs() <= 1500 && o.3.abs() <= 1500)
        .collect();
    let m = kept.len() as f64;
    let (a, b2, aa, bb, ab) = kept.iter().fold((0f64, 0f64, 0f64, 0f64, 0f64), |acc, o| {
        let (x, y) = (o.2 as f64, o.3 as f64);
        (
            acc.0 + x,
            acc.1 + y,
            acc.2 + x * x,
            acc.3 + y * y,
            acc.4 + x * y,
        )
    });
    let trimmed = (m * ab - a * b2) / ((m * aa - a * a).sqrt() * (m * bb - b2 * b2).sqrt());
    println!(
        "   correlation with both scores within ±1500: {trimmed:.3} over {} records",
        kept.len()
    );
    outliers.sort_by_key(|o| std::cmp::Reverse(o.0));
    println!("   largest differences (label, fresh re-search, depth):");
    for o in outliers.iter().take(8) {
        println!("     {:>6} {:>6} d{:<3} {}", o.2, o.3, o.4, o.1);
    }
    (
        trimmed,
        same_sign as f64 / signed.max(1) as f64,
        quantile(&mut depths.clone(), 0.5) as f64,
    )
}

fn determinism(dir: &Path) {
    use rusty_chess_tools::datagen::{Settings, play_game};
    use rusty_chess_tools::openings::Rng;
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let sh = shards
        .iter()
        .find(|s| s.name.ends_with("-s00000"))
        .expect("a shard with sequence 0 (s00000)");
    println!(
        "   the data's engine_sha: {} (build this tool there for an exact comparison)",
        sh.manifest["provenance"]["engine_sha"]
    );
    let settings: Settings = serde_json::from_value(sh.manifest["settings"].clone()).unwrap();
    let mut rng = Rng::seeded(sh.manifest["seed"].as_u64().unwrap());
    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
    s.set_hash_mb(settings.hash_mb);
    let n = 12;
    let mut same = 0;
    for (i, g) in sh.games.iter().take(n).enumerate() {
        let regen = play_game(&mut rng, &mut s, &settings);
        let recorded: Vec<&str> = g["moves"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let ok = regen.start_fen == g["start_fen"].as_str().unwrap() && regen.moves == recorded;
        if ok {
            same += 1;
        } else {
            println!(
                "   game {i} differs: start {} vs {}",
                regen.start_fen, g["start_fen"]
            );
        }
    }
    println!(
        "== Regenerated the first {n} games of {} from its seed: {same}/{n} identical (start position and every move)",
        sh.name
    );
}

/// Every record through bulletformat 1.8.0's text parser and binary writer,
/// the path `bullet-utils convert --from text` takes. Returns the failures.
fn bullet(dir: &Path) -> u64 {
    use bulletformat::{BulletFormat, ChessBoard};
    let shards = load(dir);
    let (mut lines, mut bad, mut conv_bad) = (0u64, 0u64, 0u64);
    let mut boards: Vec<ChessBoard> = Vec::new();
    for sh in &shards {
        for line in &sh.positions {
            lines += 1;
            let parts: Vec<&str> = line.split(" | ").collect();
            let (fen, ws, wr): (&str, i16, f32) = (
                parts[0],
                parts[1].parse().unwrap(),
                parts[2].parse().unwrap(),
            );
            let black = fen.split(' ').nth(1) == Some("b");
            match line.parse::<ChessBoard>() {
                Ok(bd) => {
                    let want_score = if black { -ws } else { ws };
                    let want_result = if black { 1.0 - wr } else { wr };
                    let pieces = fen
                        .split(' ')
                        .next()
                        .unwrap()
                        .chars()
                        .filter(|c| c.is_ascii_alphabetic())
                        .count();
                    if bd.score() != want_score
                        || (bd.result() - want_result).abs() > 1e-6
                        || bd.occ.count_ones() as usize != pieces
                    {
                        conv_bad += 1;
                    }
                    boards.push(bd);
                }
                Err(_) => bad += 1,
            }
        }
    }
    let path =
        std::env::temp_dir().join(format!("datagen-audit-{}.bulletformat", std::process::id()));
    {
        let mut w = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
        ChessBoard::write_to_bin(&mut w, &boards).unwrap();
    }
    let size = std::fs::metadata(&path).unwrap().len();
    let exact = size == (boards.len() * std::mem::size_of::<ChessBoard>()) as u64;
    std::fs::remove_file(&path).ok();
    println!(
        "== bulletformat 1.8.0: {lines} records; parse failures {bad}; score, result or piece count not as expected {conv_bad}; binary {size} bytes, exact {exact}"
    );
    bad + conv_bad + u64::from(!exact)
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: datagen-audit check DIR [--config datagen.toml] [--research N] [--bullet] | research DIR N | determinism DIR | outcomes DIR | thresholds DIR | adjudication DIR N | repetition DIR N | openings DIR | bullet DIR";
    let (Some(mode), Some(dir)) = (args.first(), args.get(1)) else {
        eprintln!("{usage}");
        return std::process::ExitCode::from(2);
    };
    let dir = PathBuf::from(dir);
    let has_shards = std::fs::read_dir(&dir)
        .map(|d| {
            d.flatten()
                .any(|e| e.file_name().to_string_lossy().ends_with(".json"))
        })
        .unwrap_or(false);
    if !has_shards {
        eprintln!(
            "datagen-audit: no shard manifests (*.json) in {}",
            dir.display()
        );
        return std::process::ExitCode::from(2);
    }
    let num = |i: usize| args.get(i).and_then(|v| v.parse::<usize>().ok());
    match mode.as_str() {
        "check" => {
            let mut config = PathBuf::from("datagen.toml");
            let (mut research_n, mut with_bullet) = (None, false);
            let mut i = 2;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => {
                        config = PathBuf::from(&args[i + 1]);
                        i += 2;
                    }
                    "--research" => {
                        research_n = num(i + 1);
                        i += 2;
                    }
                    "--bullet" => {
                        with_bullet = true;
                        i += 1;
                    }
                    other => {
                        eprintln!("unknown option {other}\n{usage}");
                        return std::process::ExitCode::from(2);
                    }
                }
            }
            let bc = bars(&config);
            let (mut p0, mut warns) = replay(&dir, &bc);
            if let Some(n) = research_n {
                println!();
                let (corr, sign, depth) = research(&dir, n);
                println!("\n== P1 label bars (fresh re-search)");
                bar(
                    "score correlation within ±1500",
                    corr,
                    bc.research_correlation_min,
                    f64::MAX,
                    &mut warns,
                );
                bar(
                    "same sign where |label| >= 50",
                    sign,
                    bc.research_same_sign_min,
                    f64::MAX,
                    &mut warns,
                );
                bar(
                    "median depth at the node budget",
                    depth,
                    bc.research_median_depth_min,
                    f64::MAX,
                    &mut warns,
                );
            }
            if with_bullet {
                println!();
                let bad = bullet(&dir);
                println!(
                    "  {} bulletformat parse and conventions: {bad}",
                    if bad == 0 { "PASS" } else { "FAIL" }
                );
                p0 += bad;
            }
            println!(
                "\n== Verdict: {} ({p0} P0 failures, {warns} P1 warnings)",
                if p0 > 0 {
                    "STOP"
                } else if warns > 0 {
                    "INVESTIGATE"
                } else {
                    "GO"
                }
            );
            if p0 > 0 {
                std::process::ExitCode::from(1)
            } else {
                std::process::ExitCode::SUCCESS
            }
        }
        "research" => {
            research(&dir, num(2).unwrap_or(2000));
            std::process::ExitCode::SUCCESS
        }
        "determinism" => {
            determinism(&dir);
            std::process::ExitCode::SUCCESS
        }
        "outcomes" => {
            outcomes(&dir);
            std::process::ExitCode::SUCCESS
        }
        "thresholds" => {
            thresholds(&dir);
            std::process::ExitCode::SUCCESS
        }
        "adjudication" => {
            adjudication(&dir, num(2).unwrap_or(200));
            std::process::ExitCode::SUCCESS
        }
        "repetition" => {
            repetition(&dir, num(2).unwrap_or(300));
            std::process::ExitCode::SUCCESS
        }
        "openings" => {
            openings(&dir);
            std::process::ExitCode::SUCCESS
        }
        "bullet" => {
            if bullet(&dir) == 0 {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::from(1)
            }
        }
        _ => {
            eprintln!("{usage}");
            std::process::ExitCode::from(2)
        }
    }
}

// ---------- Investigations ----------

/// Replay a game; returns the board before each engine move (with history),
/// via a callback, and the final board.
fn replay_game(g: &Value, mut each: impl FnMut(usize, &Board, i32)) -> Board {
    let start = Position::from_fen(g["start_fen"].as_str().unwrap()).unwrap();
    let mut b = Board::from_position(start);
    let moves: Vec<&str> = g["moves"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let scores: Vec<i32> = g["scores"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_i64().unwrap() as i32)
        .collect();
    for (i, uci) in moves.iter().enumerate() {
        each(i, &b, scores[i]);
        let m = b.position().parse_uci_move(uci).unwrap();
        b.make_move(m);
    }
    b
}

fn category(term: &str) -> &'static str {
    match term {
        "win-adjudication" => "win-adjudicated",
        "draw-adjudication" => "draw-adjudicated",
        "repetition" => "repetition",
        "checkmate" => "checkmate",
        _ => "other natural draw",
    }
}

fn outcomes(dir: &Path) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let edges = [-700, -300, -100, -25, 25, 100, 300, 700];
    let label = |s: i32| -> usize { edges.iter().position(|&e| s < e).unwrap_or(edges.len()) };
    let names = [
        "< -700",
        "-700..-300",
        "-300..-100",
        "-100..-25",
        "-25..25",
        "25..100",
        "100..300",
        "300..700",
        "> 700",
    ];
    // category -> bucket -> (count, sum of white result)
    let mut t: BTreeMap<&str, [(u64, f64); 9]> = BTreeMap::new();
    let mut all: Vec<(i32, f64, &str)> = Vec::new();
    for sh in &shards {
        let mut idx = 0usize;
        for g in &sh.games {
            let cat = category(g["termination"].as_str().unwrap());
            let result = g["result"].as_f64().unwrap();
            // Records of this game: as many as the replay keeps (validated by `replay`).
            let mut n = 0;
            replay_game(g, |i, b, s| {
                let pos = b.position();
                let m = pos.parse_uci_move(g["moves"][i].as_str().unwrap()).unwrap();
                if !pos.in_check() && !m.is_capture() && !m.is_promotion() && !is_mate_score(s) {
                    n += 1;
                }
            });
            for line in &sh.positions[idx..idx + n] {
                let w: i32 = line.split(" | ").nth(1).unwrap().parse().unwrap();
                let e = &mut t.entry(cat).or_insert([(0, 0.0); 9])[label(w)];
                e.0 += 1;
                e.1 += result;
                all.push((w, result, cat));
            }
            idx += n;
        }
    }
    println!(
        "== Mean game result (white's view) by score bucket (white's view), records per category"
    );
    print!("{:>12}", "bucket");
    for c in t.keys() {
        print!(" | {:>22}", c);
    }
    println!();
    for (bi, name) in names.iter().enumerate() {
        print!("{name:>12}");
        for v in t.values() {
            let (n, s) = v[bi];
            if n == 0 {
                print!(" | {:>22}", "-");
            } else {
                print!(" | {:>8.3} ({:>9})", s / n as f64, n);
            }
        }
        println!();
    }
    // Logistic fit: result ≈ sigmoid(score / K), by cross-entropy, per subset.
    let fit = |pred: &dyn Fn(&&(i32, f64, &str)) -> bool| -> (f64, f64, usize) {
        let xs: Vec<&(i32, f64, &str)> = all.iter().filter(|r| pred(r)).collect();
        let mut best = (0.0, f64::MAX);
        for k in (50..=1500).step_by(10) {
            let k = k as f64;
            let ce: f64 = xs
                .iter()
                .map(|(s, r, _)| {
                    let p = (1.0 / (1.0 + (-(*s as f64) / k).exp())).clamp(1e-6, 1.0 - 1e-6);
                    -(r * p.ln() + (1.0 - r) * (1.0 - p).ln())
                })
                .sum::<f64>()
                / xs.len() as f64;
            if ce < best.1 {
                best = (k, ce);
            }
        }
        (best.0, best.1, xs.len())
    };
    println!("\n== Best logistic scale K (result ≈ 1 / (1 + e^(-score/K))) and its cross-entropy");
    for (name, f) in [
        (
            "all records",
            Box::new(|_: &&(i32, f64, &str)| true) as Box<dyn Fn(&&(i32, f64, &str)) -> bool>,
        ),
        (
            "naturally ended games",
            Box::new(|r: &&(i32, f64, &str)| r.2 != "win-adjudicated" && r.2 != "draw-adjudicated"),
        ),
        (
            "win-adjudicated games",
            Box::new(|r: &&(i32, f64, &str)| r.2 == "win-adjudicated"),
        ),
    ] {
        let (k, ce, n) = fit(&*f);
        println!("   {name:24} K = {k:>5.0}  cross-entropy {ce:.4}  ({n} records)");
    }
}

fn adjudication(dir: &Path, n: usize) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let dataset = dataset_net(&shards);
    let net = &dataset;
    let games: Vec<&Value> = shards
        .iter()
        .flat_map(|s| s.games.iter())
        .filter(|g| g["termination"] == "win-adjudication")
        .collect();
    let step = (games.len() / n).max(1);
    let picked: Vec<&Value> = games.iter().step_by(step).take(n).copied().collect();
    let results: Vec<(f64, i32, f64, u32, &'static str)> = std::thread::scope(|scope| {
        let chunks: Vec<Vec<&Value>> = picked
            .chunks(picked.len().div_ceil(8))
            .map(|c| c.to_vec())
            .collect();
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    let mut out = Vec::new();
                    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
                    s.set_hash_mb(64);
                    for g in chunk {
                        let adj = g["result"].as_f64().unwrap();
                        let mut b = replay_game(g, |_, _, _| {});
                        b.set_network(net.clone());
                        // The adjudication fired on the search before the last move;
                        // the final board is after it. Deep search there.
                        s.clear_hash();
                        let r = s.search(
                            &mut b,
                            &Limits {
                                nodes: Some(2_000_000),
                                ..Limits::default()
                            },
                            &mut |_| {},
                        );
                        let deep_white = if b.position().side_to_move() == Color::White {
                            r.score
                        } else {
                            -r.score
                        };
                        // Play it out at 20k nodes a move, natural endings only.
                        let mut plies = 0u32;
                        let mut s2 = Searcher::new(Arc::new(AtomicBool::new(false)));
                        s2.set_hash_mb(16);
                        let outcome = loop {
                            let pos = b.position().clone();
                            let mut list = MoveList::new();
                            pos.legal_moves(&mut list);
                            if list.is_empty() {
                                break if pos.in_check() {
                                    (
                                        if pos.side_to_move() == Color::White {
                                            0.0
                                        } else {
                                            1.0
                                        },
                                        "mate",
                                    )
                                } else {
                                    (0.5, "stalemate")
                                };
                            }
                            if b.is_repetition() || b.is_fifty_moves() || insufficient(&pos) {
                                break (0.5, "draw");
                            }
                            if plies >= 300 {
                                break (-1.0, "unfinished");
                            }
                            let r = s2.search(
                                &mut b,
                                &Limits {
                                    nodes: Some(20_000),
                                    ..Limits::default()
                                },
                                &mut |_| {},
                            );
                            b.make_move(r.best);
                            plies += 1;
                        };
                        out.push((adj, deep_white, outcome.0, plies, outcome.1));
                    }
                    out
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let n = results.len() as u64;
    let deep_agree = results
        .iter()
        .filter(|(a, d, ..)| (*a == 1.0 && *d >= 300) || (*a == 0.0 && *d <= -300))
        .count() as u64;
    let deep_flip = results
        .iter()
        .filter(|(a, d, ..)| (*a == 1.0 && *d <= -100) || (*a == 0.0 && *d >= 100))
        .count() as u64;
    let deep_unclear = n - deep_agree - deep_flip;
    let finished: Vec<_> = results.iter().filter(|r| r.2 >= 0.0).collect();
    let same = finished.iter().filter(|r| r.0 == r.2).count() as u64;
    let drawn = finished.iter().filter(|r| r.2 == 0.5).count() as u64;
    let reversed = finished.iter().filter(|r| r.2 != 0.5 && r.0 != r.2).count() as u64;
    let mut dabs: Vec<i64> = results
        .iter()
        .map(|r| {
            if r.0 == 1.0 {
                r.1 as i64
            } else {
                -(r.1 as i64)
            }
        })
        .collect();
    println!(
        "== {} win-adjudicated games (of {}), checked from the position where adjudication ended them",
        n,
        games.len()
    );
    println!(
        "   deep search (2M nodes, ~{}x the game's budget), winner's view: p10 {} median {} p90 {}",
        2_000_000 / 5000,
        quantile(&mut dabs, 0.1),
        quantile(&mut dabs, 0.5),
        quantile(&mut dabs, 0.9)
    );
    println!(
        "   still winning (>= +300 for the adjudicated winner): {:.1}%   unclear (-100..300): {:.1}%   reversed (<= -100): {:.1}%",
        pct(deep_agree, n),
        pct(deep_unclear, n),
        pct(deep_flip, n)
    );
    println!(
        "   played out at 20k nodes a move: finished {} of {}; same result {:.1}%, drawn {:.1}%, reversed {:.1}%",
        finished.len(),
        n,
        pct(same, finished.len() as u64),
        pct(drawn, finished.len() as u64),
        pct(reversed, finished.len() as u64)
    );
    let mut pl: Vec<i64> = finished.iter().map(|r| r.3 as i64).collect();
    println!(
        "   playout length: median {} p90 {}",
        quantile(&mut pl, 0.5),
        quantile(&mut pl, 0.9)
    );
}

fn repetition(dir: &Path, n: usize) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let dataset = dataset_net(&shards);
    let net = &dataset;
    let games: Vec<&Value> = shards
        .iter()
        .flat_map(|s| s.games.iter())
        .filter(|g| g["termination"] == "repetition")
        .collect();
    // From the scores alone, over every repetition game.
    let (mut before300, mut before100) = (0u64, 0u64);
    let mut lens: Vec<i64> = Vec::new();
    for g in &games {
        let sc: Vec<i64> = g["scores"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_i64().unwrap())
            .collect();
        lens.push(sc.len() as i64);
        let tail = &sc[sc.len().saturating_sub(10)..];
        if tail.iter().any(|s| s.abs() >= 300) {
            before300 += 1;
        }
        if tail.iter().any(|s| s.abs() >= 100) {
            before100 += 1;
        }
    }
    let total = games.len() as u64;
    println!(
        "== {} repetition games: length median {}; |score| >= 100 somewhere in the last 10 plies {:.1}%, >= 300 {:.1}%",
        total,
        quantile(&mut lens, 0.5),
        pct(before100, total),
        pct(before300, total)
    );
    // Deep check: the position before the repeating move, with the game's history.
    let step = (games.len() / n).max(1);
    let picked: Vec<&Value> = games.iter().step_by(step).take(n).copied().collect();
    let res: Vec<(i32, bool)> = std::thread::scope(|scope| {
        let chunks: Vec<Vec<&Value>> = picked
            .chunks(picked.len().div_ceil(8))
            .map(|c| c.to_vec())
            .collect();
        let hs: Vec<_> = chunks
            .into_iter()
            .map(|chunk| {
                scope.spawn(move || {
                    let mut s = Searcher::new(Arc::new(AtomicBool::new(false)));
                    s.set_hash_mb(64);
                    let mut out = Vec::new();
                    for g in chunk {
                        let moves = g["moves"].as_array().unwrap();
                        let last = moves.len() - 1;
                        let start = Position::from_fen(g["start_fen"].as_str().unwrap()).unwrap();
                        let mut b = Board::from_position(start);
                        b.set_network(net.clone());
                        for m in &moves[..last] {
                            let mv = b.position().parse_uci_move(m.as_str().unwrap()).unwrap();
                            b.make_move(mv);
                        }
                        s.clear_hash();
                        let r = s.search(
                            &mut b,
                            &Limits {
                                nodes: Some(2_000_000),
                                ..Limits::default()
                            },
                            &mut |_| {},
                        );
                        out.push((r.score, r.best.to_string() == moves[last].as_str().unwrap()));
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    let k = res.len() as u64;
    let same = res.iter().filter(|r| r.1).count() as u64;
    let mut abs: Vec<i64> = res.iter().map(|r| r.0 as i64).collect();
    let better = res.iter().filter(|r| r.0 >= 50).count() as u64;
    let worse = res.iter().filter(|r| r.0 <= -50).count() as u64;
    println!(
        "   deep search (2M nodes) before the repeating move, {} games: same move {:.1}%; score for the side that repeated p10 {} median {} p90 {}",
        k,
        pct(same, k),
        quantile(&mut abs, 0.1),
        quantile(&mut abs, 0.5),
        quantile(&mut abs, 0.9)
    );
    println!(
        "   repeater could have kept >= +50: {:.1}%; was worse off (<= -50, so the draw was a save): {:.1}%; within ±50: {:.1}%",
        pct(better, k),
        pct(worse, k),
        pct(k - better - worse, k)
    );
}

fn openings(dir: &Path) {
    rusty_chess_core::attacks::init();
    let shards = load(dir);
    let games: Vec<&Value> = shards.iter().flat_map(|s| s.games.iter()).collect();
    let n = games.len() as u64;
    let mut pawns: HashSet<(u64, u64)> = HashSet::new();
    let mut placements: HashSet<String> = HashSet::new();
    let (mut lost_castling, mut king_moved, mut imbalanced, mut full_material, mut queen_out) =
        (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut at: BTreeMap<usize, HashSet<String>> = BTreeMap::new();
    let mut first_moves: HashMap<String, u64> = HashMap::new();
    for g in &games {
        let fen = g["start_fen"].as_str().unwrap();
        let p = Position::from_fen(fen).unwrap();
        let wp = (p.by_kind(PieceKind::Pawn) & p.by_color(Color::White)).0;
        let bp = (p.by_kind(PieceKind::Pawn) & p.by_color(Color::Black)).0;
        pawns.insert((wp, bp));
        placements.insert(fen.split(' ').next().unwrap().to_string());
        let castle = fen.split(' ').nth(2).unwrap();
        if castle != "KQkq" {
            lost_castling += 1;
        }
        let kings_home = fen.split(' ').next().unwrap();
        let rows: Vec<&str> = kings_home.split('/').collect();
        if !rows[7].contains('K') || !rows[0].contains('k') {
            king_moved += 1;
        }
        let count = |c: Color, k: PieceKind| (p.by_kind(k) & p.by_color(c)).count() as i32;
        let mat = |c: Color| {
            count(c, PieceKind::Pawn)
                + 3 * (count(c, PieceKind::Knight) + count(c, PieceKind::Bishop))
                + 5 * count(c, PieceKind::Rook)
                + 9 * count(c, PieceKind::Queen)
        };
        if mat(Color::White) != mat(Color::Black) {
            imbalanced += 1;
        }
        if p.occupied().count() == 32 {
            full_material += 1;
        }
        if !(rows[7].contains('Q') && rows[0].contains('q')) {
            queen_out += 1;
        }
        // Convergence: distinct positions after k engine plies.
        let mut b = Board::from_position(p);
        for (i, m) in g["moves"].as_array().unwrap().iter().enumerate().take(40) {
            if i == 0 {
                *first_moves
                    .entry(m.as_str().unwrap().to_string())
                    .or_insert(0) += 1;
            }
            let mv = b.position().parse_uci_move(m.as_str().unwrap()).unwrap();
            b.make_move(mv);
            if [10, 20, 40].contains(&(i + 1)) {
                at.entry(i + 1)
                    .or_default()
                    .insert(key4(&b.position().to_fen()));
            }
        }
    }
    println!("== Openings: {} games", n);
    println!(
        "   distinct start positions {} ({:.1}%); distinct piece placements {}; distinct pawn structures {} ({:.1}%)",
        n,
        100.0,
        placements.len(),
        pawns.len(),
        pct(pawns.len() as u64, n)
    );
    println!(
        "   full material {:.1}%; material imbalance {:.1}%; castling rights reduced {:.1}%; a king off its home rank {:.1}%; a queen off its home rank {:.1}%",
        pct(full_material, n),
        pct(imbalanced, n),
        pct(lost_castling, n),
        pct(king_moved, n),
        pct(queen_out, n)
    );
    for (k, set) in &at {
        println!(
            "   distinct positions after {k} engine plies: {} ({:.1}% of games that got that far)",
            set.len(),
            pct(set.len() as u64, n)
        );
    }
    let mut fm: Vec<(&String, &u64)> = first_moves.iter().collect();
    fm.sort_by(|a, b| b.1.cmp(a.1));
    println!(
        "   distinct first engine moves {}; the most common {} ({:.1}%)",
        fm.len(),
        fm[0].0,
        pct(*fm[0].1, n)
    );
}

/// Would a lower win-adjudication threshold have called the right winner?
/// For each threshold, find where it would first have fired (win_plies
/// consecutive plies at or beyond it for one side) and compare with how the
/// game actually ended, and count the records it would have cut.
fn thresholds(dir: &Path) {
    let shards = load(dir);
    let win_plies = shards[0].manifest["settings"]["win_plies"]
        .as_u64()
        .unwrap() as usize;
    println!(
        "== Hypothetical win adjudication at lower thresholds ({win_plies} consecutive plies), against the games' real endings"
    );
    println!(
        "{:>6} | {:>7} | {:>8} | {:>8} | {:>9} | {:>14}",
        "cp", "fires", "right", "drawn", "reversed", "plies cut"
    );
    for t in [600, 800, 1000, 1200, 1500, 2000] {
        let (mut fired, mut right, mut drawn, mut reversed, mut cut, mut total) =
            (0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
        for g in shards.iter().flat_map(|s| s.games.iter()) {
            let sc: Vec<i32> = g["scores"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_i64().unwrap() as i32)
                .collect();
            let start_white = g["start_fen"].as_str().unwrap().split(' ').nth(1) == Some("w");
            let result = g["result"].as_f64().unwrap();
            total += sc.len() as u64;
            let mut run = 0usize;
            let mut leader: Option<bool> = None;
            for (i, &s) in sc.iter().enumerate() {
                let mover_white = (i % 2 == 0) == start_white;
                if s.abs() >= t {
                    let lw = if s > 0 { mover_white } else { !mover_white };
                    run = if leader == Some(lw) { run + 1 } else { 1 };
                    leader = Some(lw);
                    if run >= win_plies {
                        fired += 1;
                        let call = if lw { 1.0 } else { 0.0 };
                        if result == call {
                            right += 1;
                        } else if result == 0.5 {
                            drawn += 1;
                        } else {
                            reversed += 1;
                        }
                        cut += (sc.len() - 1 - i) as u64;
                        break;
                    }
                } else {
                    run = 0;
                    leader = None;
                }
            }
        }
        println!(
            "{t:>6} | {fired:>7} | {:>7.2}% | {:>7.2}% | {:>8.2}% | {:>13.1}%",
            pct(right, fired),
            pct(drawn, fired),
            pct(reversed, fired),
            pct(cut, total)
        );
    }
}
