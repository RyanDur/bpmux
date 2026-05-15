//! #1299 Cassiopeia Slice E — regression tests pinning the no-parallel-
//! shape contract for iteration membership. After Slice E retired the
//! `iteration_id` field + the `iteration-#NNNN` label-emit path, the
//! only iteration-membership signal is positional (via `icebox_unified`
//! in priority.json). These tests fail if the legacy axis re-emerges
//! from any future regression.

#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::adapters::github::decode_story_metadata;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::store::BoardStore;
use crate::board::{BoardEvent, PeerId, events::EventMeta};
use chrono::{TimeZone, Utc};
use serde_json::json;

fn meta() -> EventMeta {
    EventMeta {
        actor: "test".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

#[test]
fn story_struct_has_no_iteration_id_field_in_board_json_projection() {
    let story = Story::new_minimal(
        100,
        "carrier-test".to_string(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    let json = crate::board::domain::story_to_board_json(&story);
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("iteration_id"),
        "iteration_id surface retired (#1299 Slice E); board-JSON must not advertise it: keys={:?}",
        obj.keys().collect::<Vec<_>>()
    );
}

#[test]
fn iteration_label_decoder_does_not_populate_a_story_field() {
    let v = json!({
        "number": 100,
        "title": "decoder-test",
        "state": "open",
        "created_at": "2026-05-15T00:00:00Z",
        "user": { "login": "pm" },
        "body": "fixture",
        "labels": [{"name": "ready"}, {"name": "iteration-#1338"}],
    });
    let story = decode_story_metadata(&v).expect("decode");
    let json = crate::board::domain::story_to_board_json(&story);
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("iteration_id"),
        "iteration-#NNNN label must not surface as iteration_id field"
    );
    let labels = obj
        .get("labels")
        .and_then(|v| v.as_array())
        .expect("labels array");
    let label_strs: Vec<&str> = labels.iter().filter_map(|v| v.as_str()).collect();
    assert!(
        !label_strs.iter().any(|l| l.starts_with("iteration-#")),
        "story-to-board emit no longer derives iteration-#NNNN from a field; \
         legacy labels on raw_labels are not re-emitted into wire-format-labels: {:?}",
        label_strs
    );
}

#[test]
fn create_story_event_does_not_seed_iteration_membership() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta(),
        number: 200,
        title: "freshly-created".to_string(),
        body: "no membership at filing-time".to_string(),
        initial_state: StoryState::Icebox,
        story_type: StoryType::Feature,
    });
    let s = store.story(200).expect("story present");
    let json = crate::board::domain::story_to_board_json(&s);
    let obj = json.as_object().expect("object");
    assert!(
        !obj.contains_key("iteration_id"),
        "newly-created stories surface no iteration_id"
    );
}
