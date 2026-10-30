//! Self-play data generation: settings, games, filters and shard files.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusty_chess_core::position::Position;
use rusty_chess_core::types::Color;
use rusty_chess_search::board::Board;
use rusty_chess_search::nnue::DEFAULT_NET_GENERATION;
use rusty_chess_search::search::Searcher;
use rusty_chess_tools::datagen::{
    Adjudicator, Manifest, Provenance, SCHEMA, Settings, Termination, board_termination,
    insufficient_material, parse_record, play_game, record_line, shard_name, write_shard,
};
use rusty_chess_tools::openings::Rng;

fn repo_settings() -> Settings {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datagen.toml");
    Settings::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
}

/// The committed settings, cheap enough for a test.
fn quick() -> Settings {
    Settings {
        nodes: 300,
        hash_mb: 1,
        ..repo_settings()
    }
}

fn searcher() -> Searcher {
    rusty_chess_core::attacks::init();
    Searcher::new(Arc::new(AtomicBool::new(false)))
}

#[test]
fn the_committed_settings_parse_and_bad_ones_do_not() {
    let s = repo_settings();
    assert_eq!(
        s.generation, DEFAULT_NET_GENERATION,
        "the committed settings play with the embedded net"
    );
    assert!(s.nodes >= 1000, "{s:?}");
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../datagen.toml"))
        .unwrap();
    assert!(Settings::parse(&text.replace("nodes = 5000", "nodes = 0")).is_err());
    assert!(Settings::parse(&text.replace("max_plies = 400", "max_plies = 4")).is_err());
    assert!(
        Settings::parse(&text.replace("[datagen]\n", "[datagen]\nbogus = 1\n")).is_err(),
        "unknown keys are refused"
    );
    assert!(
        Settings::parse(&text.replace("generation = 1", "generation = 7")).is_err(),
        "a generation without its net is refused"
    );
    let pesto = Settings::parse(&text.replace("generation = 1", "generation = 0")).unwrap();
    assert!(pesto.network().is_none(), "generation 0 is PeSTO");
    assert!(s.network().is_some(), "{s:?}");
}

#[test]
fn a_game_is_deterministic_for_a_seed_and_differs_across_seeds() {
    let s = quick();
    let game = |seed| play_game(&mut Rng::seeded(seed), &mut searcher(), &s);
    let (a, b, c) = (game(1), game(1), game(2));
    assert_eq!(a.moves, b.moves);
    assert_eq!(a.samples, b.samples);
    assert_ne!(a.start_fen, c.start_fen);
}

#[test]
fn kept_positions_are_quiet_and_labelled_from_whites_side() {
    let s = quick();
    let mut rng = Rng::seeded(3);
    let mut sr = searcher();
    let mut kept = 0;
    for _ in 0..4 {
        let g = play_game(&mut rng, &mut sr, &s);
        assert!([0.0, 0.5, 1.0].contains(&g.result));
        assert_eq!(g.moves.len(), g.scores.len());
        assert!(g.samples.len() <= g.moves.len());
        for sample in &g.samples {
            let pos = Position::from_fen(&sample.fen).unwrap();
            assert!(!pos.in_check(), "{}", sample.fen);
            assert!(
                sample.score_white.abs() < 20_000,
                "no mate scores: {}",
                sample.score_white
            );
            // The record round-trips.
            let line = record_line(sample, g.result);
            assert_eq!(
                parse_record(&line),
                Some((sample.fen.clone(), sample.score_white, g.result))
            );
        }
        kept += g.samples.len();
    }
    assert!(kept > 0);
    assert_eq!(parse_record("8/8/8/8/8/8/8/K6k w - - 0 1 | x | 0.5"), None);
}

#[test]
fn insufficient_material_is_bare_kings_or_one_minor() {
    let pos = |f: &str| Position::from_fen(f).unwrap();
    assert!(insufficient_material(&pos("8/8/4k3/8/8/3K4/8/8 w - - 0 1")));
    assert!(insufficient_material(&pos(
        "8/8/4k3/8/8/3K4/5N2/8 w - - 0 1"
    )));
    assert!(!insufficient_material(&pos(
        "8/8/4k3/8/8/3K4/5NN1/8 w - - 0 1"
    )));
    assert!(!insufficient_material(&pos(
        "8/8/4k3/8/8/3K4/5P2/8 w - - 0 1"
    )));
}

#[test]
fn a_shard_is_three_whole_files_whose_counts_agree() {
    let s = quick();
    let mut rng = Rng::seeded(5);
    let mut sr = searcher();
    let games: Vec<_> = (0..2).map(|_| play_game(&mut rng, &mut sr, &s)).collect();
    let dir = std::env::temp_dir().join(format!("datagen-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let name = shard_name("t", 2, 7);
    assert_eq!(name, "t-w002-s00007");
    let positions: usize = games.iter().map(|g| g.samples.len()).sum();
    let manifest = Manifest {
        schema: SCHEMA,
        shard: name.clone(),
        worker: 2,
        seq: 7,
        seed: 5,
        provenance: Provenance {
            run: "r".into(),
            task: "t".into(),
            engine_version: "0.0.0".into(),
            engine_sha: "abc".into(),
            engine_bench: None,
        },
        settings: s.clone(),
        games: 2,
        positions: positions as u64,
        results: [0, 0, 0],
        terminations: Default::default(),
        seconds: 0.0,
        files: [
            format!("{name}.positions.txt"),
            format!("{name}.games.jsonl"),
        ],
        sha256: Default::default(),
    };
    write_shard(&dir, &games, &manifest).unwrap();
    let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
    assert_eq!(read(&manifest.files[0]).lines().count(), positions);
    assert_eq!(read(&manifest.files[1]).lines().count(), 2);
    let m: serde_json::Value = serde_json::from_str(&read(&format!("{name}.json"))).unwrap();
    assert_eq!(m["schema"], SCHEMA);
    assert_eq!(m["settings"]["nodes"], 300);
    // No temporary files are left behind.
    assert!(
        std::fs::read_dir(&dir).unwrap().all(|e| !e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with('.'))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

fn board_at(fen: &str) -> Board {
    rusty_chess_core::attacks::init();
    Board::from_position(Position::from_fen(fen).unwrap())
}

#[test]
fn every_board_ending_is_recognised_with_the_right_result() {
    let s = repo_settings();
    let end = |fen: &str, plies: u32| board_termination(&board_at(fen), plies, &s);
    // White mated: black wins.
    assert_eq!(
        end(
            "rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3",
            5
        ),
        Some((0.0, Termination::Checkmate))
    );
    // Black mated: white wins.
    assert_eq!(
        end(
            "r1bqkb1r/pppp1Qpp/2n2n2/4p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4",
            5
        ),
        Some((1.0, Termination::Checkmate))
    );
    assert_eq!(
        end("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", 5),
        Some((0.5, Termination::Stalemate))
    );
    assert_eq!(
        end("8/8/4k3/8/8/3K4/3R4/8 w - - 100 80", 5),
        Some((0.5, Termination::FiftyMoves))
    );
    assert_eq!(
        end("8/8/4k3/8/8/3K4/3N4/8 w - - 0 1", 5),
        Some((0.5, Termination::InsufficientMaterial))
    );
    let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    assert_eq!(end(start, s.max_plies), Some((0.5, Termination::MaxPlies)));
    assert_eq!(end(start, 0), None);
    // A repetition: the knights go out and back.
    let mut b = board_at(start);
    for m in ["g1f3", "g8f6", "f3g1", "f6g8"] {
        let mv = b.position().parse_uci_move(m).unwrap();
        b.make_move(mv);
    }
    assert_eq!(
        board_termination(&b, 4, &s),
        Some((0.5, Termination::Repetition))
    );
}

#[test]
fn win_adjudication_needs_consecutive_plies_for_one_side() {
    let s = repo_settings();
    let w = s.win_cp;
    // White leads on four consecutive plies, seen from each mover's side.
    let mut a = Adjudicator::default();
    let plies = [
        (Color::White, w + 300),
        (Color::Black, -w - 300),
        (Color::White, w),
        (Color::Black, -w - 50),
    ];
    for (i, (stm, score)) in plies.iter().enumerate() {
        let got = a.update(*stm, *score, 10 + i as u32, &s);
        if i + 1 < s.win_plies as usize {
            assert_eq!(got, None, "ply {i}");
        } else {
            assert_eq!(got, Some((1.0, Termination::WinAdjudication)));
        }
    }
    // A change of leader or a ply below the threshold restarts the count.
    let mut a = Adjudicator::default();
    for (stm, score) in [
        (Color::White, w),
        (Color::Black, w),
        (Color::White, -w),
        (Color::Black, w - 1),
        (Color::White, -w),
        (Color::Black, w),
        (Color::White, -w),
    ] {
        assert_eq!(a.update(stm, score, 20, &s), None);
    }
    assert_eq!(
        a.update(Color::Black, w, 21, &s),
        Some((0.0, Termination::WinAdjudication)),
        "black led four plies running"
    );
    // A mate score counts as a win.
    let mut a = Adjudicator::default();
    let mate = 29_000;
    let got: Vec<_> = (0..s.win_plies)
        .map(|i| {
            a.update(
                if i % 2 == 0 {
                    Color::White
                } else {
                    Color::Black
                },
                if i % 2 == 0 { -mate } else { mate },
                30 + i,
                &s,
            )
        })
        .collect();
    assert_eq!(
        got.last().copied().flatten(),
        Some((0.0, Termination::WinAdjudication))
    );
}

#[test]
fn draw_adjudication_needs_the_ply_threshold_and_consecutive_level_scores() {
    let s = repo_settings();
    // Before draw_after_ply nothing counts.
    let mut a = Adjudicator::default();
    for p in 1..s.draw_after_ply {
        assert_eq!(a.update(Color::White, 0, p, &s), None);
    }
    // From there, draw_plies consecutive level scores end it; a bigger score restarts.
    let mut a = Adjudicator::default();
    let start = s.draw_after_ply;
    for i in 0..s.draw_plies - 1 {
        assert_eq!(a.update(Color::White, s.draw_cp, start + i, &s), None);
    }
    assert_eq!(
        a.update(Color::White, s.draw_cp + 1, start + s.draw_plies, &s),
        None
    );
    for i in 0..s.draw_plies - 1 {
        assert_eq!(a.update(Color::White, -s.draw_cp, start + 20 + i, &s), None);
    }
    assert_eq!(
        a.update(Color::White, 0, start + 40, &s),
        Some((0.5, Termination::DrawAdjudication))
    );
}

#[test]
fn random_plies_must_be_even_and_openings_start_with_white_to_move() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../datagen.toml"))
        .unwrap();
    assert!(Settings::parse(&text.replace("random_plies = 8", "random_plies = 7")).is_err());
    let s = quick();
    let mut rng = Rng::seeded(9);
    let mut sr = searcher();
    for _ in 0..6 {
        let g = play_game(&mut rng, &mut sr, &s);
        assert_eq!(g.start_fen.split(' ').nth(1), Some("w"), "{}", g.start_fen);
    }
}

#[test]
fn a_manifest_carries_the_sha256_of_each_record_file() {
    use sha2::Digest;
    let s = quick();
    let mut rng = Rng::seeded(4);
    let games = vec![play_game(&mut rng, &mut searcher(), &s)];
    let dir = std::env::temp_dir().join(format!("datagen-sha-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let name = shard_name("t", 0, 0);
    let manifest = Manifest {
        schema: SCHEMA,
        shard: name.clone(),
        worker: 0,
        seq: 0,
        seed: 4,
        provenance: Provenance {
            run: "r".into(),
            task: "t".into(),
            engine_version: "0".into(),
            engine_sha: "x".into(),
            engine_bench: None,
        },
        settings: s,
        games: 1,
        positions: games[0].samples.len() as u64,
        results: [0, 0, 0],
        terminations: Default::default(),
        seconds: 0.0,
        files: [
            format!("{name}.positions.txt"),
            format!("{name}.games.jsonl"),
        ],
        sha256: Default::default(),
    };
    write_shard(&dir, &games, &manifest).unwrap();
    let m: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{name}.json"))).unwrap())
            .unwrap();
    for (i, f) in manifest.files.iter().enumerate() {
        let want: String = sha2::Sha256::digest(std::fs::read(dir.join(f)).unwrap())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(m["sha256"][i], want, "{f}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
