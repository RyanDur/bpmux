#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::json::story_to_board_json;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{Story, StoryId, StoryKind, StoryState, StoryType};

fn sample_story(state: StoryState, story_type: StoryType, is_closed: bool) -> Story {
    let mut s = Story {
        id: StoryId::new(42),
        title: "[Feature] sample".to_string(),
        state,
        filed_at: chrono::DateTime::parse_from_rfc3339("2026-04-26T01:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        filed_by: "pm:test".to_string(),
        body: Some("body content".to_string()),
        free_comments: Some(Vec::new()),
        gate_comments: Some(Vec::new()),
        is_closed,
        raw_labels: Vec::new(),
        body_etag: None,
        story_type,
        acceptor: None,
        parent_epic: None,
        iteration_membership: None,
        tags: Vec::new(),
        ready_order: None,
    };
    s.raw_labels
        .push(GhLabelCodec.encode_story_type(story_type));
    s
}

#[test]
fn story_to_board_json_emits_renderer_wire_format_field_list() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);

    let obj = json.as_object().expect("object");
    let keys: Vec<&String> = obj.keys().collect();
    for required in &[
        "id",
        "title",
        "state",
        "gh_state",
        "labels",
        "parent_epic",
        "iteration_membership",
        "outcome_membership",
        "iteration_order",
        "iteration_review_closed_by",
        "iteration_dev_ok_by",
        "horizon",
        "is_current",
        "acceptor",
        "assignees",
        "created_at",
        "body",
        "rework_comment",
    ] {
        assert!(
            keys.iter().any(|k| k.as_str() == *required),
            "wire-format must include `{required}` (drift catches #451-class regression); keys present: {keys:?}"
        );
    }
    assert_eq!(obj.len(), 18, "no spurious keys; keys present: {keys:?}");
}

#[test]
fn story_to_board_json_iteration_membership_null_when_none_1492() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(
        json["iteration_membership"].is_null(),
        "iteration_membership emits JSON null when Story.iteration_membership is None; got: {}",
        json["iteration_membership"]
    );
}

#[test]
fn story_to_board_json_iteration_membership_object_when_set_1492() {
    let mut s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.raw_labels.push("parent-iteration-#1464.100".to_string());
    s.iteration_membership = Some((IterationId::new(1464), 100));
    let json = story_to_board_json(&GhLabelCodec, &s);
    let membership = json["iteration_membership"]
        .as_object()
        .expect("iteration_membership object when Some");
    assert_eq!(membership["iteration_id"].as_u64(), Some(1464));
    assert_eq!(membership["position"].as_u64(), Some(100));
}

#[test]
fn story_to_board_json_iteration_order_null_when_none_1735() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(
        json["iteration_order"].is_null(),
        "iteration_order emits JSON null when no iteration-order label is present; got: {}",
        json["iteration_order"]
    );
}

#[test]
fn story_to_board_json_iteration_order_number_when_set_1735() {
    let mut s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.raw_labels = vec![GhLabelCodec.encode_iteration_order(200)];
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert_eq!(
        json["iteration_order"].as_u64(),
        Some(200),
        "iteration_order serializes as the canonical position when set"
    );
}

#[test]
fn story_to_board_json_acceptor_null_when_none_1551() {
    let s = sample_story(
        StoryState::Delivered,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(
        json["acceptor"].is_null(),
        "acceptor emits JSON null when unassigned (honest-neutral, not a type-guess); got: {}",
        json["acceptor"]
    );
}

#[test]
fn story_to_board_json_acceptor_emits_class_string_1551() {
    let mut s = sample_story(
        StoryState::Delivered,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.raw_labels.push("awaiting dev".to_string());
    s.acceptor = Some(crate::product_management::story::Acceptor::Dev);
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert_eq!(
        json["acceptor"].as_str(),
        Some("dev"),
        "acceptor emits the canonical class string for the renderer"
    );
}

#[test]
fn story_to_board_json_rework_comment_null_when_no_backward_edge() {
    let s = sample_story(
        StoryState::Ready,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(json["rework_comment"].is_null());
}

#[test]
fn story_to_board_json_rework_comment_carries_backward_edge_payload() {
    use crate::product_management::story::{Gate, GateComment};
    let mut s = sample_story(
        StoryState::Ready,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.gate_comments = Some(vec![
        GateComment {
            gate: Gate::Pickup,
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Delivered,
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: Some("ATP green; user-job verified".into()),
        },
        GateComment {
            gate: Gate::PassDeskCheck,
            ts: chrono::Utc::now(),
            actor: "designer".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Rejected,
            ts: chrono::Utc::now(),
            actor: "owner".into(),
            payload: Some("user-story not satisfied".into()),
        },
    ]);
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert_eq!(
        json["rework_comment"].as_str(),
        Some("user-story not satisfied"),
        "rework_comment carries the backward transition's comment"
    );
}

#[test]
fn story_to_board_json_rework_comment_clears_after_forward_edge() {
    use crate::product_management::story::{Gate, GateComment};
    let mut s = sample_story(
        StoryState::InProgress,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.gate_comments = Some(vec![
        GateComment {
            gate: Gate::Pickup,
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Delivered,
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::PassDeskCheck,
            ts: chrono::Utc::now(),
            actor: "designer".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Rejected,
            ts: chrono::Utc::now(),
            actor: "owner".into(),
            payload: Some("rework needed".into()),
        },
        GateComment {
            gate: Gate::Pickup,
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
    ]);
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(
        json["rework_comment"].is_null(),
        "forward transition (Pickup after Rejected) suppresses rework_comment"
    );
}

#[test]
fn story_to_board_json_emits_canonical_6_state_vocabulary() {
    for (state, expected) in &[
        (StoryState::Icebox, "icebox"),
        (StoryState::Ready, "ready"),
        (StoryState::InProgress, "in-progress"),
        (StoryState::DeskCheck, "desk-check"),
        (StoryState::Delivered, "delivered"),
        (StoryState::Accepted, "accepted"),
    ] {
        let s = sample_story(*state, StoryType::Story(StoryKind::Feature), false);
        assert_eq!(
            json_str(&story_to_board_json(&GhLabelCodec, &s), "state"),
            *expected,
            "{state:?} serializes to canonical wire-vocab `{expected}` per #505 boundary-principle"
        );
    }
}

#[test]
fn story_to_board_json_is_closed_emits_gh_state_closed_independent_of_state() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        true,
    );
    assert_eq!(
        json_str(&story_to_board_json(&GhLabelCodec, &s), "state"),
        "icebox",
        "Story.state stays canonical regardless of gh-side is_closed; the GH-adapter is_closed→Accepted translation happens at INGEST not at serialization"
    );
    assert_eq!(
        json_str(&story_to_board_json(&GhLabelCodec, &s), "gh_state"),
        "closed"
    );
}

#[test]
fn story_to_board_json_gh_state_lowercase_open_or_closed() {
    let open = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    assert_eq!(
        json_str(&story_to_board_json(&GhLabelCodec, &open), "gh_state"),
        "open"
    );
    let closed = sample_story(
        StoryState::Accepted,
        StoryType::Story(StoryKind::Feature),
        true,
    );
    assert_eq!(
        json_str(&story_to_board_json(&GhLabelCodec, &closed), "gh_state"),
        "closed"
    );
}

#[test]
fn story_to_board_json_labels_include_story_type() {
    for (story_type, expected_label) in &[
        (StoryType::Story(StoryKind::Feature), "feature"),
        (StoryType::Story(StoryKind::Bug), "bug"),
        (StoryType::Story(StoryKind::Chore), "chore"),
        (StoryType::Epic, "epic"),
    ] {
        let s = sample_story(StoryState::Icebox, *story_type, false);
        let json = story_to_board_json(&GhLabelCodec, &s);
        let labels = json["labels"].as_array().expect("labels array");
        assert!(
            labels.iter().any(|l| l.as_str() == Some(expected_label)),
            "labels include `{expected_label}` for {story_type:?}"
        );
    }
}

#[test]
fn story_to_board_json_labels_include_blocked_tag() {
    let mut s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.tags = vec![crate::product_management::story::Tag::Blocked];
    let json = story_to_board_json(&GhLabelCodec, &s);
    let labels = json["labels"].as_array().expect("labels array");
    assert!(labels.iter().any(|l| l.as_str() == Some("blocked")));
}

#[test]
fn story_to_board_json_parent_epic_emits_field_and_label() {
    let mut s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.parent_epic = Some(EpicId::new(7));
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert_eq!(json["parent_epic"].as_u64(), Some(7));
    let labels = json["labels"].as_array().expect("labels array");
    assert!(
        labels.iter().any(|l| l.as_str() == Some("parent-epic-#7")),
        "labels include `parent-epic-#7`; got {labels:?}"
    );
}

#[test]
fn story_to_board_json_no_parent_epic_emits_null() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert!(
        json["parent_epic"].is_null(),
        "parent_epic null when absent; got {:?}",
        json["parent_epic"]
    );
}

#[test]
fn story_to_board_json_body_unwraps_to_empty_string_when_none() {
    let mut s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.body = None;
    let json = story_to_board_json(&GhLabelCodec, &s);
    assert_eq!(json["body"].as_str(), Some(""));
}

#[test]
fn story_to_board_json_assignees_empty_array() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    let assignees = json["assignees"].as_array().expect("assignees array");
    assert!(assignees.is_empty());
}

#[test]
fn story_to_board_json_created_at_is_rfc3339() {
    let s = sample_story(
        StoryState::Icebox,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    let json = story_to_board_json(&GhLabelCodec, &s);
    let created = json["created_at"].as_str().expect("created_at string");
    chrono::DateTime::parse_from_rfc3339(created).expect("created_at parses as RFC3339");
}

fn json_str<'a>(json: &'a serde_json::Value, key: &str) -> &'a str {
    json[key].as_str().unwrap_or_else(|| {
        panic!("key `{key}` not a string in {json}");
    })
}

struct StubLabelCodec;

impl crate::product_management::board::label_codec::LabelCodec for StubLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("rdy:")?.parse().ok()
    }
    fn decode_iteration(
        &self,
        label: &str,
    ) -> Option<crate::product_management::iteration::label::IterationLabelParts> {
        let rest = label.strip_prefix("iter|")?;
        let (name, pos_str) = rest.rsplit_once(':')?;
        let position = pos_str.parse().ok()?;
        if name.is_empty() {
            return None;
        }
        Some(
            crate::product_management::iteration::label::IterationLabelParts {
                name: name.to_string(),
                position,
            },
        )
    }
    fn is_iteration_label_loose(&self, label: &str) -> bool {
        label.starts_with("iter|") && label.contains(':')
    }
    fn encode_parent_epic(&self, issue_number: u64) -> String {
        format!("parent|{issue_number}")
    }
    fn encode_ready_order(&self, position: u32) -> String {
        format!("rdy:{position}")
    }
    fn encode_iteration(&self, name: &str, position: u32) -> String {
        format!("iter|{name}:{position}")
    }
    fn iteration_label_class(&self, label: &str) -> Option<String> {
        let parts = self.decode_iteration(label)?;
        Some(format!("iter|{}:", parts.name))
    }
}

#[test]
fn substrate_test_story_to_board_json_works_with_arbitrary_codec_syntax() {
    let mut s = sample_story(
        StoryState::Ready,
        StoryType::Story(StoryKind::Feature),
        false,
    );
    s.parent_epic = Some(EpicId::new(42));
    s.ready_order = Some(100);
    s.raw_labels = vec!["iter|Pegasus:200".to_string(), "noise-label".to_string()];
    let json = story_to_board_json(&StubLabelCodec, &s);
    let labels: Vec<&str> = json["labels"]
        .as_array()
        .expect("labels array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        labels.contains(&"parent|42"),
        "stub-encoded parent-epic surfaces in wire labels: {labels:?}"
    );
    assert!(
        labels.contains(&"iter|Pegasus:200"),
        "stub-iteration-like label forwarded: {labels:?}"
    );
    assert!(
        labels.contains(&"rdy:100"),
        "stub ready-order forwarded: {labels:?}"
    );
    assert!(
        !labels.contains(&"noise-label"),
        "non-iteration non-ready-order labels not forwarded: {labels:?}"
    );
}
