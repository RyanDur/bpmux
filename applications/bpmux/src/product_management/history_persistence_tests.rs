#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::history::{SharedHistory, react_to_story};
use super::history_persistence;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{StoryId, Verdict};

fn fresh_dir(tag: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("bpmux-shared-history-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn one_verdict() -> SharedHistory {
    react_to_story(
        &SharedHistory::default(),
        &StoryAction::Event(StoryEvent::new(
            EventMeta {
                actor: "owner".to_string(),
                ts: Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
                peer_id: PeerId::local(),
            },
            StoryId::new(700),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Succeed {
                    step: "acceptance".into(),
                },
                payload: Some("evidence".to_string()),
            },
        )),
    )
    .expect("a cast verdict appends a record")
}

#[test]
fn shared_history_round_trips_through_the_json_file() {
    let dir = fresh_dir("roundtrip");
    let history = one_verdict();
    history_persistence::save(&dir, &history).unwrap();
    let loaded = history_persistence::load(&dir);
    assert_eq!(loaded.records(), history.records());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_shared_history_file_loads_as_empty() {
    let dir = fresh_dir("missing");
    assert!(history_persistence::load(&dir).records().is_empty());
}

#[test]
fn a_corrupt_shared_history_file_loads_as_empty() {
    let dir = fresh_dir("corrupt");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(history_persistence::shared_history_path(&dir), "{ not json").unwrap();
    assert!(history_persistence::load(&dir).records().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
