#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::usage::{Record, Source, key_for, last_used_datetime, load_from, parse_key, save_to};
use std::collections::HashMap;
use tempfile::TempDir;

fn tmp_path() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("usage.json");
    (dir, path)
}

#[test]
fn key_for_prefixes_source() {
    assert_eq!(key_for(Source::Slash, "/board"), "slash:/board");
    assert_eq!(key_for(Source::Mcp, "accept_story"), "mcp:accept_story");
}

#[test]
fn parse_key_extracts_source_and_feature() {
    let (source, feature) = parse_key("slash:/board").expect("parse slash");
    assert_eq!(source, Source::Slash);
    assert_eq!(feature, "/board");
    let (source, feature) = parse_key("mcp:accept_story").expect("parse mcp");
    assert_eq!(source, Source::Mcp);
    assert_eq!(feature, "accept_story");
}

#[test]
fn parse_key_returns_none_for_unknown_prefix() {
    assert!(parse_key("unknown:/board").is_none());
    assert!(parse_key("no-colon").is_none());
}

#[test]
fn save_and_load_roundtrip() {
    let (_dir, path) = tmp_path();
    let mut map = HashMap::new();
    map.insert(
        "mcp:accept_story".to_string(),
        Record {
            count: 5,
            last_used: "2026-04-16T12:00:00+00:00".to_string(),
        },
    );
    save_to(&path, &map).expect("save");
    let loaded = load_from(&path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded.get("mcp:accept_story").unwrap().count, 5);
}

#[test]
fn load_from_missing_file_returns_empty() {
    let (_dir, path) = tmp_path();
    let loaded = load_from(&path);
    assert!(loaded.is_empty());
}

#[test]
fn load_from_corrupt_file_returns_empty() {
    let (_dir, path) = tmp_path();
    std::fs::write(&path, "not valid json {{{").unwrap();
    let loaded = load_from(&path);
    assert!(loaded.is_empty(), "corrupt file treated as empty");
}

#[test]
fn load_from_legacy_format_survives() {
    let (_dir, path) = tmp_path();
    let legacy = r#"{"slash:/board": [3, "2026-04-10T10:00:00+00:00"]}"#;
    std::fs::write(&path, legacy).unwrap();
    let loaded = load_from(&path);
    assert_eq!(loaded.len(), 1);
    let rec = loaded.get("slash:/board").expect("legacy key loaded");
    assert_eq!(rec.count, 3);
    assert!(rec.last_used.contains("2026-04-10"));
}

#[test]
fn last_used_datetime_parses_roundtrip() {
    let rec = Record::now(1);
    assert!(
        last_used_datetime(&rec).is_some(),
        "Record::now should emit parseable RFC3339"
    );
}

#[test]
fn save_then_reload_preserves_counts_across_restarts() {
    let (_dir, path) = tmp_path();
    let mut map = HashMap::new();
    map.insert("slash:/view".to_string(), Record::now(7));
    save_to(&path, &map).expect("save");

    let loaded = load_from(&path);
    assert_eq!(loaded.get("slash:/view").unwrap().count, 7);
}
