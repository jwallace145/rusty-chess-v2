//! Self-play data generation: settings, games, filters and shard files.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use rusty_chess_core::position::Position;
use rusty_chess_search::search::Searcher;
use rusty_chess_tools::datagen::{
    Manifest, Provenance, SCHEMA, Settings, insufficient_material, parse_record, play_game,
    record_line, shard_name, write_shard,
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
    assert_eq!(s.generation, 0);
    assert!(s.nodes >= 1000, "{s:?}");
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../datagen.toml"))
        .unwrap();
    assert!(Settings::parse(&text.replace("nodes = 5000", "nodes = 0")).is_err());
    assert!(Settings::parse(&text.replace("max_plies = 400", "max_plies = 4")).is_err());
    assert!(
        Settings::parse(&format!("{text}\nbogus = 1\n")).is_err(),
        "unknown keys are refused"
    );
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
