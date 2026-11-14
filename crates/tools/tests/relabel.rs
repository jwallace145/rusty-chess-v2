//! Relabelling: the same records in the same order with new scores, matched
//! against the shard's games, deterministic, and refusing anything that
//! doesn't match.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use braveheart_core::types::Color;
use braveheart_search::search::{MATE, Searcher};
use braveheart_tools::datagen::{Settings, play_game, record_line};
use braveheart_tools::openings::Rng;
use braveheart_tools::relabel::{MATE_CLAMP, relabel_shard, white_label};

fn quick() -> Settings {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../datagen.toml");
    Settings {
        nodes: 300,
        hash_mb: 1,
        ..Settings::parse(&std::fs::read_to_string(path).unwrap()).unwrap()
    }
}

fn searcher() -> Searcher {
    braveheart_core::attacks::init();
    Searcher::new(Arc::new(AtomicBool::new(false)))
}

/// A small shard as datagen writes it: (records file, games file).
fn shard(games: usize, seed: u64) -> (String, String) {
    let s = quick();
    let (mut rng, mut sr) = (Rng::seeded(seed), searcher());
    let (mut positions, mut records) = (String::new(), String::new());
    for _ in 0..games {
        let g = play_game(&mut rng, &mut sr, &s);
        for sample in &g.samples {
            positions.push_str(&record_line(sample, g.result));
            positions.push('\n');
        }
        records.push_str(&serde_json::to_string(&g).unwrap());
        records.push('\n');
    }
    (positions, records)
}

fn split(line: &str) -> (String, i32, String) {
    let mut p = line.split(" | ");
    let fen = p.next().unwrap().to_string();
    let score = p.next().unwrap().parse().unwrap();
    (fen, score, p.next().unwrap().to_string())
}

#[test]
fn a_relabelled_shard_keeps_every_record_in_order_and_is_deterministic() {
    let (positions, games) = shard(2, 7);
    let s = quick();
    let (a, stats) = relabel_shard(&positions, &games, &mut searcher(), s.network(), 300).unwrap();
    let old: Vec<_> = positions.lines().map(split).collect();
    let new: Vec<_> = a.lines().map(split).collect();
    assert!(!old.is_empty());
    assert_eq!(old.len(), new.len());
    assert_eq!(stats.records, old.len() as u64);
    for (o, n) in old.iter().zip(&new) {
        assert_eq!((&o.0, &o.2), (&n.0, &n.2), "FEN and result are kept");
    }
    // Same net and budget: the scores track the originals (a flipped point
    // of view would disagree in sign almost everywhere).
    let agree = old
        .iter()
        .zip(&new)
        .filter(|(o, n)| (o.1 > 0) == (n.1 > 0) || o.1 == 0 || n.1 == 0)
        .count();
    assert!(
        agree * 10 >= old.len() * 8,
        "{agree} of {} agree in sign",
        old.len()
    );
    // A fresh searcher, or one that relabelled something else first, gives
    // the same file: each game starts from a cleared table and history.
    let mut used = searcher();
    let (other_positions, other_games) = shard(1, 11);
    relabel_shard(&other_positions, &other_games, &mut used, s.network(), 300).unwrap();
    let (b, _) = relabel_shard(&positions, &games, &mut used, s.network(), 300).unwrap();
    assert_eq!(a, b);
}

#[test]
fn a_record_that_is_in_no_game_is_refused() {
    let (positions, games) = shard(1, 3);
    let mut lines: Vec<String> = positions.lines().map(String::from).collect();
    let mid = lines.len() / 2;
    let (fen, score, result) = split(&lines[mid]);
    // Change the half-move clock: a FEN the game never reached.
    let mut parts: Vec<&str> = fen.split(' ').collect();
    let clock = (parts[4].parse::<u32>().unwrap() + 7).to_string();
    parts[4] = &clock;
    lines[mid] = format!("{} | {score} | {result}", parts.join(" "));
    let tampered = lines.join("\n") + "\n";
    let err =
        relabel_shard(&tampered, &games, &mut searcher(), quick().network(), 300).unwrap_err();
    assert!(err.contains("is in no game"), "{err}");
}

#[test]
fn a_record_whose_result_is_not_its_games_is_refused() {
    let (positions, games) = shard(1, 5);
    let mut lines: Vec<String> = positions.lines().map(String::from).collect();
    let (fen, score, result) = split(&lines[0]);
    let other = if result == "1.0" { "0.0" } else { "1.0" };
    lines[0] = format!("{fen} | {score} | {other}");
    let tampered = lines.join("\n") + "\n";
    let err =
        relabel_shard(&tampered, &games, &mut searcher(), quick().network(), 300).unwrap_err();
    assert!(err.contains("ended"), "{err}");
}

#[test]
fn labels_are_from_whites_view_and_mate_scores_are_clamped() {
    assert_eq!(white_label(120, Color::White), (120, false));
    assert_eq!(white_label(120, Color::Black), (-120, false));
    assert_eq!(white_label(MATE - 5, Color::White), (MATE_CLAMP, true));
    assert_eq!(white_label(MATE - 5, Color::Black), (-MATE_CLAMP, true));
    assert_eq!(white_label(-(MATE - 9), Color::White), (-MATE_CLAMP, true));
}

fn sha256(text: &str) -> String {
    use sha2::Digest;
    sha2::Sha256::digest(text)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Generation 0's manifests predate `sha256`: a relabelled shard's manifest
/// has both checksums either way, in `files` order, for the audit to check.
#[test]
fn a_relabelled_manifest_has_both_checksums_even_if_the_original_had_none() {
    use std::io::Read;
    let dir = std::env::temp_dir().join(format!("relabel-sha-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let task = std::path::Path::new("gen=0/run=r/task=t0");
    let source = dir.join("in").join(task);
    for sub in ["manifests", "positions", "games"] {
        std::fs::create_dir_all(source.join(sub)).unwrap();
    }
    let (positions, games) = shard(1, 5);
    let count = positions.lines().count();
    let without = serde_json::json!({"positions": count});
    let with =
        serde_json::json!({"positions": count, "sha256": [sha256(&positions), sha256(&games)]});
    for (stem, manifest) in [("t0-w000-s00000", without), ("t0-w000-s00001", with)] {
        std::fs::write(
            source.join("manifests").join(format!("{stem}.json")),
            manifest.to_string(),
        )
        .unwrap();
        std::fs::write(
            source.join("positions").join(format!("{stem}.txt")),
            &positions,
        )
        .unwrap();
        std::fs::write(source.join("games").join(format!("{stem}.jsonl")), &games).unwrap();
    }

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_relabel"))
        .args(["--net", "default", "--nodes", "300", "--threads", "1"])
        .arg("--in")
        .arg(dir.join("in"))
        .arg("--out")
        .arg(dir.join("out"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let out = dir.join("out").join(task);
    for stem in ["t0-w000-s00000", "t0-w000-s00001"] {
        let m: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(out.join("manifests").join(format!("{stem}.json"))).unwrap(),
        )
        .unwrap();
        let mut records = String::new();
        flate2::read::GzDecoder::new(
            std::fs::File::open(out.join("positions").join(format!("{stem}.txt.gz"))).unwrap(),
        )
        .read_to_string(&mut records)
        .unwrap();
        assert_eq!(
            m["sha256"],
            serde_json::json!([sha256(&records), sha256(&games)]),
            "{stem}"
        );
        assert_eq!(
            m["relabel"]["source_positions_sha256"],
            sha256(&positions),
            "{stem}"
        );
    }
    std::fs::remove_dir_all(&dir).unwrap();
}
