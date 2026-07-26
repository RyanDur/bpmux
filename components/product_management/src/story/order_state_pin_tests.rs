#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};

use crate::epic::EpicId;
use crate::story::{Acceptor, Story, StoryId, Tag, Work};

const CANONICAL_STORY_KEYS: &[&str] = &[
    "id",
    "title",
    "state",
    "filed_at",
    "filed_by",
    "body",
    "free_comments",
    "verdicts",
    "body_etag",
    "story_type",
    "acceptor",
    "parent_epic",
    "tags",
    "remote_updated_at",
    "state_written_at",
];

const SELF_FILE: &str = "order_state_pin_tests.rs";

const ORDER_FIELD_MARKERS: &[&str] = &[
    "ready_order:",
    ".ready_order",
    "\"ready_order\"",
    "iteration_slot:",
    ".iteration_slot",
    "\"iteration_slot\"",
];

fn fully_populated_story() -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(1),
        "[Feature] a story that owns no order".to_string(),
        Work::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    story.body = Some("body".to_string());
    story.free_comments = Some(Vec::new());
    story.verdicts = Some(Vec::new());
    story.body_etag = Some("etag".to_string());
    story.acceptor = Some(Acceptor::Owner);
    story.parent_epic = Some(EpicId::new(7));
    story.tags = vec![Tag::DeskCheckFail];
    story.remote_updated_at = Some(Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap());
    story.state_written_at = Some(Utc.with_ymd_and_hms(2026, 1, 3, 0, 0, 0).unwrap());
    story
}

#[test]
fn story_serializes_to_a_fixed_key_set_carrying_no_order_field() {
    let value = serde_json::to_value(fully_populated_story()).expect("story serializes");
    let object = value
        .as_object()
        .expect("a story serializes to a JSON object");

    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    let mut expected: Vec<&str> = CANONICAL_STORY_KEYS.to_vec();
    expected.sort_unstable();

    assert_eq!(
        keys, expected,
        "containers own order, the story never does (Ryan, 2026-07-07): the Story wire shape is exactly {expected:?}. A new key here means an order/position/slot field (ready_order, iteration_slot, ...) was added to the Story — order and membership are the iteration/board container's concern (iteration.stories, ready-queue ordering), projected in the view, not stored on the story. Move the field onto the container. Serialized keys were {keys:?}."
    );
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read src dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn is_test_source(path: &Path, contents: &str) -> bool {
    path.to_string_lossy().contains("_tests")
        || contents.contains("#[test]")
        || contents.contains("#[tokio::test]")
}

#[test]
fn no_order_or_position_field_lives_in_production() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        if is_test_source(file, &contents) {
            continue;
        }
        for marker in ORDER_FIELD_MARKERS {
            if contents.contains(marker) {
                violations.push(format!("{}: {marker}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "containers own order, the story never does (Ryan, 2026-07-07): no production type carries a ready_order or iteration_slot field (declaration, access, or serde key). Ordering lives on the container (the ready queue, iteration membership). A field-shaped occurrence in production is order state creeping back onto an entity — put it on the container. Order/position fields in production:\n{}",
        violations.join("\n")
    );
}
