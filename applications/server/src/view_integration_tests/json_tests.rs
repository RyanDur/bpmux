#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use product_management::epic::EpicId;
use product_management::story::{Story, StoryId, StoryKind, Work};
use source_codec::gh_label_codec::GhLabelCodec;
use view::board::story_to_board_json;

fn sample_story(state: Work, story_type: StoryKind) -> Story {
    Story {
        id: StoryId::new(42),
        title: "[Feature] sample".to_string(),
        state,
        filed_at: chrono::DateTime::parse_from_rfc3339("2026-04-26T01:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        filed_by: "pm:test".to_string(),
        body: Some("body content".to_string()),
        free_comments: Some(Vec::new()),
        verdicts: Some(Vec::new()),
        body_etag: None,
        story_type,
        acceptor: None,
        parent_epic: None,
        tags: Vec::new(),
        remote_updated_at: None,
        state_written_at: None,
    }
}

#[test]
fn story_to_board_json_emits_renderer_wire_format_field_list() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());

    let obj = json.as_object().expect("object");
    let keys: Vec<&String> = obj.keys().collect();
    for required in &[
        "id",
        "title",
        "state",
        "labels",
        "parent_epic",
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
    assert_eq!(obj.len(), 10, "no spurious keys; keys present: {keys:?}");
    assert!(
        !keys.iter().any(|k| k.as_str() == "gh_state"),
        "gh_state sheds from the story wire — closedness derives from state at every reader; keys present: {keys:?}"
    );
    assert!(
        !keys.iter().any(|k| k.as_str() == "iteration_membership"),
        "iteration_membership is gone from the wire — membership is the iteration container's concern (iteration.stories); keys present: {keys:?}"
    );
}

#[test]
fn story_to_board_json_acceptor_null_when_none_1551() {
    let s = sample_story(Work::Acceptance, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert!(
        json["acceptor"].is_null(),
        "acceptor emits JSON null when unassigned (honest-neutral, not a type-guess); got: {}",
        json["acceptor"]
    );
}

#[test]
fn story_to_board_json_acceptor_emits_class_string_1551() {
    let mut s = sample_story(Work::Acceptance, StoryKind::Feature);
    s.acceptor = Some(product_management::story::Acceptor::Dev);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(
        json["acceptor"].as_str(),
        Some("dev"),
        "acceptor emits the canonical class string for the renderer"
    );
}

#[test]
fn story_to_board_json_rework_comment_null_when_no_backward_edge() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert!(json["rework_comment"].is_null());
}

#[test]
fn story_to_board_json_rework_comment_carries_backward_edge_payload() {
    use product_management::story::{CastVerdict, Verdict};
    let mut s = sample_story(Work::NotStarted, StoryKind::Feature);
    s.verdicts = Some(vec![
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "in-progress".into(),
            },
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: Some("ATP green; user-job verified".into()),
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "desk-check".into(),
            },
            ts: chrono::Utc::now(),
            actor: "designer".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Fail {
                step: "acceptance".into(),
            },
            ts: chrono::Utc::now(),
            actor: "owner".into(),
            payload: Some("user-story not satisfied".into()),
        },
    ]);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(
        json["rework_comment"].as_str(),
        Some("user-story not satisfied"),
        "rework_comment carries the backward transition's comment"
    );
}

#[test]
fn story_to_board_json_rework_comment_clears_after_forward_edge() {
    use product_management::story::{CastVerdict, Verdict};
    let mut s = sample_story(Work::InProgress, StoryKind::Feature);
    s.verdicts = Some(vec![
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "in-progress".into(),
            },
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "desk-check".into(),
            },
            ts: chrono::Utc::now(),
            actor: "designer".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Fail {
                step: "acceptance".into(),
            },
            ts: chrono::Utc::now(),
            actor: "owner".into(),
            payload: Some("rework needed".into()),
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: chrono::Utc::now(),
            actor: "dev".into(),
            payload: None,
        },
    ]);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert!(
        json["rework_comment"].is_null(),
        "forward transition (Pickup after Rejected) suppresses rework_comment"
    );
}

#[test]
fn story_to_board_json_emits_canonical_6_state_vocabulary() {
    for (state, projected) in &[
        (Work::NotStarted, "icebox"),
        (Work::NotStarted, "ready"),
        (Work::InProgress, "in-progress"),
        (Work::Acceptance, "acceptance"),
        (Work::Accepted, "accepted"),
        (Work::Retired, "retired"),
    ] {
        let s = sample_story(state.clone(), StoryKind::Feature);
        assert_eq!(
            json_str(&story_to_board_json(&GhLabelCodec, &s, projected), "state"),
            *projected,
            "the projected `{projected}` state serializes onto the wire verbatim; the GitHub label stays `delivered` at the edge"
        );
    }
}

#[test]
fn story_to_board_json_state_stays_canonical_regardless_of_conclusion() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    assert_eq!(
        json_str(&story_to_board_json(&GhLabelCodec, &s, "icebox"), "state"),
        "icebox",
        "Story.state stays canonical; the GH-adapter is_closed→Accepted translation happens at INGEST not at serialization"
    );
}

#[test]
fn story_to_board_json_labels_include_story_type() {
    for (story_type, expected_label) in &[
        (StoryKind::Feature, "feature"),
        (StoryKind::Bug, "bug"),
        (StoryKind::Chore, "chore"),
    ] {
        let s = sample_story(Work::NotStarted, *story_type);
        let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
        let labels = json["labels"].as_array().expect("labels array");
        assert!(
            labels.iter().any(|l| l.as_str() == Some(expected_label)),
            "labels include `{expected_label}` for {story_type:?}"
        );
    }
}

#[test]
fn story_to_board_json_labels_include_desk_check_fail_tag() {
    let mut s = sample_story(Work::NotStarted, StoryKind::Feature);
    s.tags = vec![product_management::story::Tag::DeskCheckFail];
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    let labels = json["labels"].as_array().expect("labels array");
    assert!(labels.iter().any(|l| l.as_str() == Some("desk-check-fail")));
}

#[test]
fn a_legacy_ready_order_label_ingested_does_not_resurface_on_the_board_wire() {
    let json = serde_json::json!({
        "number": 8,
        "title": "[Feature] legacy ready-order carrier",
        "state": "open",
        "gh_state": "open",
        "labels": [{"name": "feature"}, {"name": "ready-order.5"}],
        "created_at": "2026-07-07T00:00:00Z",
        "user": {"login": "pm"},
    });
    let story = source_codec::mapper::decode_story_metadata(&json).expect("issue JSON must decode");
    assert!(
        story.tags().is_empty(),
        "a legacy ready-order label must never land in the typed tags surface"
    );
    let wire = story_to_board_json(&GhLabelCodec, &story, story.state.as_str());
    let labels: Vec<&str> = wire["labels"]
        .as_array()
        .expect("labels array")
        .iter()
        .filter_map(|l| l.as_str())
        .collect();
    assert!(
        !labels.iter().any(|l| l.starts_with("ready-order.")),
        "a legacy ready-order label is dropped at ingest and never round-trips back onto the board wire: {labels:?}"
    );
}

#[test]
fn story_to_board_json_parent_epic_emits_field_and_label() {
    let mut s = sample_story(Work::NotStarted, StoryKind::Feature);
    s.parent_epic = Some(EpicId::new(7));
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(json["parent_epic"].as_u64(), Some(7));
    let labels = json["labels"].as_array().expect("labels array");
    assert!(
        labels.iter().any(|l| l.as_str() == Some("parent-epic-#7")),
        "labels include `parent-epic-#7`; got {labels:?}"
    );
}

#[test]
fn story_to_board_json_no_parent_epic_emits_null() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert!(
        json["parent_epic"].is_null(),
        "parent_epic null when absent; got {:?}",
        json["parent_epic"]
    );
}

#[test]
fn story_to_board_json_body_unwraps_to_empty_string_when_none() {
    let mut s = sample_story(Work::NotStarted, StoryKind::Feature);
    s.body = None;
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    assert_eq!(json["body"].as_str(), Some(""));
}

#[test]
fn story_to_board_json_assignees_empty_array() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    let assignees = json["assignees"].as_array().expect("assignees array");
    assert!(assignees.is_empty());
}

#[test]
fn story_to_board_json_created_at_is_rfc3339() {
    let s = sample_story(Work::NotStarted, StoryKind::Feature);
    let json = story_to_board_json(&GhLabelCodec, &s, s.state.as_str());
    let created = json["created_at"].as_str().expect("created_at string");
    chrono::DateTime::parse_from_rfc3339(created).expect("created_at parses as RFC3339");
}

fn json_str<'a>(json: &'a serde_json::Value, key: &str) -> &'a str {
    json[key].as_str().unwrap_or_else(|| {
        panic!("key `{key}` not a string in {json}");
    })
}

struct StubLabelCodec;

impl product_management::label_codec::LabelCodec for StubLabelCodec {
    fn decode_iteration(
        &self,
        label: &str,
    ) -> Option<product_management::iteration::label::IterationLabelParts> {
        let rest = label.strip_prefix("iter|")?;
        let (name, pos_str) = rest.rsplit_once(':')?;
        let position = pos_str.parse().ok()?;
        if name.is_empty() {
            return None;
        }
        Some(product_management::iteration::label::IterationLabelParts {
            name: name.to_string(),
            position,
        })
    }
    fn is_iteration_label_loose(&self, label: &str) -> bool {
        label.starts_with("iter|") && label.contains(':')
    }
    fn encode_parent_epic(&self, issue_number: u64) -> String {
        format!("parent|{issue_number}")
    }
    fn encode_iteration(&self, name: &str, position: u32) -> String {
        format!("iter|{name}:{position}")
    }
    fn iteration_label_class(&self, label: &str) -> Option<String> {
        let parts = self.decode_iteration(label)?;
        Some(format!("iter|{}:", parts.name))
    }
    fn encode_tag(&self, tag: product_management::story::Tag) -> &'static str {
        match tag {
            product_management::story::Tag::DeskCheckFail => "stub-desk-check-fail",
        }
    }
    fn encode_acceptor(&self, acceptor: product_management::story::Acceptor) -> &'static str {
        match acceptor {
            product_management::story::Acceptor::Team => "stub-team",
            product_management::story::Acceptor::Owner => "stub-owner",
            product_management::story::Acceptor::Dev => "stub-dev",
        }
    }
    fn stage_label(&self, _stage: product_management::story::Stage) -> &'static str {
        "stub-stage"
    }
}

#[test]
fn substrate_test_story_to_board_json_works_with_arbitrary_codec_syntax() {
    let mut s = sample_story(Work::NotStarted, StoryKind::Feature);
    s.parent_epic = Some(EpicId::new(42));
    let json = story_to_board_json(&StubLabelCodec, &s, s.state.as_str());
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
}
