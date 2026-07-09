#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::history::{PlacementHistory, react_to_story};
use super::history_persistence;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, StoryId, Work};

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "bpmux-placement-history-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn one_placement() -> PlacementHistory {
    react_to_story(
        &PlacementHistory::default(),
        &StoryAction::Event(StoryEvent::new(
            EventMeta {
                actor: "dev".to_string(),
                ts: Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
            },
            StoryId::new(700),
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: Some(Stage::DeskCheck.id()),
            },
        )),
    )
    .expect("a placement-bearing transition appends a record")
}

#[test]
fn placement_history_round_trips_through_the_json_file() {
    let dir = fresh_dir("roundtrip");
    let history = one_placement();
    history_persistence::save(&dir, &history).unwrap();
    let loaded = history_persistence::load(&dir);
    assert_eq!(loaded.records(), history.records());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_placement_history_file_loads_as_empty() {
    let dir = fresh_dir("missing");
    assert!(history_persistence::load(&dir).records().is_empty());
}

#[test]
fn a_corrupt_placement_history_file_loads_as_empty() {
    let dir = fresh_dir("corrupt");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        history_persistence::placement_history_path(&dir),
        "{ not json",
    )
    .unwrap();
    assert!(history_persistence::load(&dir).records().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
