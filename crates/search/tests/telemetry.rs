//! JSON lines telemetry: one record per line, versioned, round-trippable.

use rusty_chess_search::telemetry::{
    BenchRecord, IterRecord, JsonlWriter, Record, SCHEMA_VERSION, SummaryRecord, read_records,
};

fn iter() -> IterRecord {
    IterRecord {
        v: SCHEMA_VERSION,
        fen: "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".into(),
        depth: 9,
        nodes: 412_331,
        qnodes: 198_220,
        nps: 2_310_000,
        ebf: Some(2.1),
        fh_first: Some(0.91),
        tt_hit: Some(0.38),
        score_cp: 34,
        pv: "e2e4 e7e5".into(),
    }
}

#[test]
fn schema_version_is_one() {
    assert_eq!(SCHEMA_VERSION, 1);
}

#[test]
fn an_iter_record_is_one_line_with_type_and_version_first() {
    let mut out = Vec::new();
    JsonlWriter::new(&mut out)
        .write(&Record::Iter(iter()))
        .unwrap();
    let line = String::from_utf8(out).unwrap();
    assert_eq!(line.matches('\n').count(), 1);
    assert!(line.ends_with('\n'));
    assert!(line.starts_with(r#"{"type":"iter","v":1,"#), "{line}");
    assert!(line.contains(r#""fen":"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1""#));
    assert!(line.contains(r#""ebf":2.1"#));
    assert!(line.contains(r#""pv":"e2e4 e7e5""#));
}

#[test]
fn missing_rates_serialise_as_null_so_the_schema_is_stable() {
    let mut rec = iter();
    rec.ebf = None;
    rec.fh_first = None;
    rec.tt_hit = None;
    let mut out = Vec::new();
    JsonlWriter::new(&mut out)
        .write(&Record::Iter(rec))
        .unwrap();
    let line = String::from_utf8(out).unwrap();
    assert!(
        line.contains(r#""ebf":null,"fh_first":null,"tt_hit":null"#),
        "{line}"
    );
}

#[test]
fn records_round_trip_through_a_reader() {
    let records = vec![
        Record::Iter(iter()),
        Record::Summary(SummaryRecord {
            v: SCHEMA_VERSION,
            fen: "8/8/8/8/8/8/8/K6k w - - 0 1".into(),
            depth: 9,
            nodes: 412_331,
            qnodes: 198_220,
            nps: 2_310_000,
            elapsed_ms: 178,
            bestmove: "e2e4".into(),
            score_cp: 34,
            stats: None,
        }),
        Record::Bench(BenchRecord {
            v: SCHEMA_VERSION,
            positions: 50,
            depth: 12,
            hash_mb: 16,
            cpu: None,
            nodes: 12_345_678,
            nps: 2_000_000,
            elapsed_ms: 6_172,
            stats: None,
        }),
    ];
    let mut out = Vec::new();
    {
        let mut w = JsonlWriter::new(&mut out);
        for r in &records {
            w.write(r).unwrap();
        }
    }
    let back = read_records(out.as_slice()).unwrap();
    assert_eq!(back, records);
}

#[test]
fn reader_skips_blank_lines_and_rejects_garbage() {
    let text = "\n{\"type\":\"bench\",\"v\":1,\"positions\":1,\"depth\":1,\"hash_mb\":1,\"nodes\":0,\"nps\":0,\"elapsed_ms\":0,\"stats\":null}\n\n";
    assert_eq!(read_records(text.as_bytes()).unwrap().len(), 1);
    assert!(read_records("not json\n".as_bytes()).is_err());
}

#[test]
fn bench_record_carries_the_stats_summary_when_present() {
    let text = r#"{"type":"bench","v":1,"positions":1,"depth":1,"hash_mb":1,"nodes":0,"nps":0,"elapsed_ms":0,"stats":{"nodes":5,"qnodes":1,"qnode_share":0.2,"tt_probes":10,"tt_hits":5,"tt_cutoffs":0,"tt_collisions":0,"tt_hit":0.5,"fail_highs":3,"fh_first":0.6666666666666666,"cutoffs_by_stage":[0,0,1,0,0],"null_move_cutoffs":0,"lmr_reductions":0,"lmr_researches":0,"futility_skips":0}}"#;
    let records = read_records(text.as_bytes()).unwrap();
    match &records[0] {
        Record::Bench(b) => {
            let s = b.stats.as_ref().unwrap();
            assert_eq!(s.nodes, 5);
            assert_eq!(s.cutoffs_by_stage[2], 1);
        }
        other => panic!("unexpected record {other:?}"),
    }
}
