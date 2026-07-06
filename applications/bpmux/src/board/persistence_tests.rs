#![allow(clippy::unwrap_used)]
use super::{BoardAction, BoardPlacements, persistence, reduce};
use crate::product_management::story::StoryId;

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bpmux-placements-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn placements_round_trip_through_the_json_file() {
    let dir = fresh_dir("roundtrip");
    let placed = reduce(
        &BoardPlacements::default(),
        &BoardAction::StoryPlaced {
            story: StoryId::new(42),
            stage: "desk-check".into(),
        },
    )
    .unwrap();
    persistence::save(&dir, &placed).unwrap();
    let loaded = persistence::load(&dir);
    assert_eq!(loaded.pairs(), placed.pairs());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_file_loads_as_empty() {
    let dir = fresh_dir("missing");
    assert!(persistence::load(&dir).pairs().is_empty());
}
