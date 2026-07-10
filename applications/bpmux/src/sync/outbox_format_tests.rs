#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use crate::product_management::epic::EpicId;
use crate::product_management::story::events::{
    EventMeta, LabelDerivedFields, StoryEvent, StoryEventKind,
};
use crate::product_management::story::{
    Acceptor, CastVerdict, FreeComment, Stage, Story, StoryId, StoryKind, Tag, Verdict, Work,
};
use crate::sync::{QueuedEvent, load_outbox_from_disk};

const FIX_QUEUED_TAG_ADDED: &str = r#"{"id":0,"event":{"TagAdded":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"label":"ready"}}}"#;
const FIX_QUEUED_TRANSITIONED: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"new_state":"in-progress","placement":"desk-check"}}}"#;
const FIX_QUEUED_VERDICT: &str = r#"{"id":0,"event":{"VerdictCast":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"verdict":{"Fail":{"step":"acceptance"}},"payload":"narrow viewport"}}}"#;
const FIX_QUEUED_REFRESHED: &str = r#"{"id":0,"event":{"StoryRefreshed":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"story":{"id":700,"title":"fixture","state":"not-started","filed_at":"2026-07-01T00:00:00Z","filed_by":"pm","body":null,"free_comments":null,"verdicts":null,"body_etag":null,"story_type":"Feature","acceptor":null,"parent_epic":null,"tags":[]}}}}"#;
const FIX_STORY_EVENT: &str = r#"{"TagAdded":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"label":"ready"}}"#;

const FIX_ABSENT_PLACEMENT: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z"},"id":700,"new_state":"in-progress"}}}"#;
const FIX_ABSENT_PLACEMENT_NORMALIZED: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"new_state":"in-progress","placement":null}}}"#;
const FIX_ABSENT_ETAG: &str = r#"{"id":0,"event":{"BodyFetched":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z"},"id":700,"body":"hydrated"}}}"#;
const FIX_ABSENT_ETAG_NORMALIZED: &str = r#"{"id":0,"event":{"BodyFetched":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"body":"hydrated","etag":null}}}"#;
const FIX_ABSENT_ACTOR_ID: &str = r#"{"id":0,"event":{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z"},"id":700,"label":"ready"}}}"#;
const FIX_ABSENT_ACTOR_ID_NORMALIZED: &str = r#"{"id":0,"event":{"TagAdded":{"meta":{"actor":"dev","actor_id":null,"ts":"2026-07-07T12:00:00Z"},"id":700,"label":"ready"}}}"#;
const FIX_LEGACY_ROW_WITH_PEER_ID: &str = r#"{"id":0,"event":{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"label":"ready"}}}"#;

fn fixture_meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
    }
}

fn fixture_story() -> Story {
    Story::new_minimal(
        StoryId::new(700),
        "fixture".to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn fixture_tag_added() -> StoryEvent {
    StoryEvent::new(
        fixture_meta(),
        StoryId::new(700),
        StoryEventKind::TagAdded {
            label: "ready".to_string(),
        },
    )
}

fn assert_byte_round_trip(fixture: &str) {
    let parsed: QueuedEvent = serde_json::from_str(fixture).expect("persisted row deserializes");
    let reserialized = serde_json::to_string(&parsed).expect("row reserializes");
    assert_eq!(
        reserialized, fixture,
        "the persisted outbox row round-trips byte-identically — any drift here means A2-0b changed the frozen on-disk codec"
    );
}

#[test]
fn queued_tag_added_row_round_trips_byte_identically() {
    assert_byte_round_trip(FIX_QUEUED_TAG_ADDED);
}

#[test]
fn queued_state_transitioned_row_round_trips_byte_identically() {
    assert_byte_round_trip(FIX_QUEUED_TRANSITIONED);
}

#[test]
fn queued_verdict_cast_row_round_trips_byte_identically() {
    assert_byte_round_trip(FIX_QUEUED_VERDICT);
}

#[test]
fn queued_story_refreshed_row_round_trips_byte_identically() {
    assert_byte_round_trip(FIX_QUEUED_REFRESHED);
}

#[test]
fn story_event_round_trips_byte_identically() {
    let parsed: StoryEvent =
        serde_json::from_str(FIX_STORY_EVENT).expect("story event deserializes");
    assert_eq!(parsed, fixture_tag_added());
    let reserialized = serde_json::to_string(&parsed).expect("story event reserializes");
    assert_eq!(reserialized, FIX_STORY_EVENT);
}

#[test]
fn a_legacy_disk_row_carrying_peer_id_still_deserializes() {
    let parsed: QueuedEvent =
        serde_json::from_str(FIX_LEGACY_ROW_WITH_PEER_ID).expect("legacy row deserializes");
    assert_eq!(parsed.id, crate::sync::MutationId(0));
    assert_eq!(parsed.event.meta.actor, "dev");
    assert_eq!(
        parsed.event.meta.ts,
        Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap()
    );
    match &parsed.event.kind {
        StoryEventKind::TagAdded { label } => assert_eq!(label, "ready"),
        other => panic!("fixture is a TagAdded row; got {other:?}"),
    }
}

#[test]
fn a_row_missing_the_placement_key_defaults_and_normalizes_on_rewrite() {
    let parsed: QueuedEvent =
        serde_json::from_str(FIX_ABSENT_PLACEMENT).expect("pre-placement row deserializes");
    match &parsed.event.kind {
        StoryEventKind::StateTransitioned { placement, .. } => assert_eq!(placement, &None),
        other => panic!("fixture is a StateTransitioned row; got {other:?}"),
    }
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FIX_ABSENT_PLACEMENT_NORMALIZED,
        "rewriting a pre-placement row lands the defaulted key explicitly — the documented normalization, not byte-preservation"
    );
}

#[test]
fn a_row_missing_the_etag_key_defaults_and_normalizes_on_rewrite() {
    let parsed: QueuedEvent =
        serde_json::from_str(FIX_ABSENT_ETAG).expect("pre-etag row deserializes");
    match &parsed.event.kind {
        StoryEventKind::BodyFetched { etag, .. } => assert_eq!(etag, &None),
        other => panic!("fixture is a BodyFetched row; got {other:?}"),
    }
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FIX_ABSENT_ETAG_NORMALIZED,
        "rewriting a pre-etag row lands the defaulted key explicitly — the documented normalization, not byte-preservation"
    );
}

#[test]
fn a_row_missing_the_actor_id_key_defaults_and_normalizes_on_rewrite() {
    let parsed: QueuedEvent =
        serde_json::from_str(FIX_ABSENT_ACTOR_ID).expect("pre-actor_id row deserializes");
    assert_eq!(parsed.event.meta.actor_id, None);
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FIX_ABSENT_ACTOR_ID_NORMALIZED,
        "rewriting a pre-actor_id row lands the defaulted key explicitly — the documented normalization, not byte-preservation"
    );
}

#[test]
fn a_current_shape_outbox_file_loads_and_rewrites_byte_identically() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board-outbox.json");
    let file_body = format!("[{FIX_QUEUED_TAG_ADDED},{FIX_QUEUED_TRANSITIONED}]");
    std::fs::write(&path, &file_body).unwrap();
    let loaded = load_outbox_from_disk(&path).expect("current file loads");
    let rewritten =
        serde_json::to_string(&loaded.iter().cloned().collect::<Vec<_>>()).expect("reserializes");
    assert_eq!(
        rewritten, file_body,
        "the whole JSON-file outbox round-trips byte-identically through load + persist serialization"
    );
}

fn variant_name(event: &StoryEvent) -> &'static str {
    match &event.kind {
        StoryEventKind::StoryCreated { .. } => "StoryCreated",
        StoryEventKind::StateTransitioned { .. } => "StateTransitioned",
        StoryEventKind::TagAdded { .. } => "TagAdded",
        StoryEventKind::TagRemoved { .. } => "TagRemoved",
        StoryEventKind::TagSet { .. } => "TagSet",
        StoryEventKind::TagUnset { .. } => "TagUnset",
        StoryEventKind::VerdictCast { .. } => "VerdictCast",
        StoryEventKind::FreeCommentAppended { .. } => "FreeCommentAppended",
        StoryEventKind::BodyFetched { .. } => "BodyFetched",
        StoryEventKind::FreeCommentsFetched { .. } => "FreeCommentsFetched",
        StoryEventKind::VerdictsFetched { .. } => "VerdictsFetched",
        StoryEventKind::StoryClosed { .. } => "StoryClosed",
        StoryEventKind::StoryReopened => "StoryReopened",
        StoryEventKind::AcceptorAssigned { .. } => "AcceptorAssigned",
        StoryEventKind::ParentEpicSet { .. } => "ParentEpicSet",
        StoryEventKind::ParentEpicUnset => "ParentEpicUnset",
        StoryEventKind::CurrentIterationDesignated => "CurrentIterationDesignated",
        StoryEventKind::CurrentIterationCleared => "CurrentIterationCleared",
        StoryEventKind::StoryRefreshed { .. } => "StoryRefreshed",
        StoryEventKind::LabelsRefreshed { .. } => "LabelsRefreshed",
        StoryEventKind::TitleUpdated { .. } => "TitleUpdated",
        StoryEventKind::BodyUpdated { .. } => "BodyUpdated",
    }
}

fn every_variant() -> Vec<StoryEvent> {
    let meta = fixture_meta;
    let id = StoryId::new(700);
    vec![
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::StoryCreated {
                title: "t".to_string(),
                body: "b".to_string(),
                initial_state: Work::NotStarted,
                story_type: StoryKind::Feature,
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: Some(Stage::DeskCheck.id()),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::TagAdded {
                label: "ready".to_string(),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::TagRemoved {
                label: "ready".to_string(),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::TagSet {
                tag: Tag::DeskCheckFail,
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::TagUnset {
                tag: Tag::DeskCheckFail,
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::VerdictCast {
                verdict: Verdict::Fail {
                    step: "acceptance".into(),
                },
                payload: Some("reason".to_string()),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::FreeCommentAppended {
                body: "comment".to_string(),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::BodyFetched {
                body: "body".to_string(),
                etag: Some("W/\"etag\"".to_string()),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::FreeCommentsFetched {
                comments: vec![FreeComment {
                    body: "c".to_string(),
                    author: "pm".to_string(),
                    ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
                }],
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::VerdictsFetched {
                verdicts: vec![CastVerdict {
                    verdict: Verdict::Succeed {
                        step: "in-progress".into(),
                    },
                    ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
                    actor: "dev".to_string(),
                    payload: None,
                }],
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::StoryClosed {
                reason: Some("done".to_string()),
            },
        ),
        StoryEvent::new(meta(), id, StoryEventKind::StoryReopened),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::AcceptorAssigned {
                acceptor: Acceptor::Team,
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::ParentEpicSet {
                epic_id: EpicId::new(9),
            },
        ),
        StoryEvent::new(meta(), id, StoryEventKind::ParentEpicUnset),
        StoryEvent::new(meta(), id, StoryEventKind::CurrentIterationDesignated),
        StoryEvent::new(meta(), id, StoryEventKind::CurrentIterationCleared),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::StoryRefreshed {
                story: Box::new(fixture_story()),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::LabelsRefreshed {
                fields: LabelDerivedFields {
                    raw_labels: vec!["feature".to_string()],
                    acceptor: Some(Acceptor::Team),
                    parent_epic: Some(EpicId::new(9)),
                    tags: vec![Tag::DeskCheckFail],
                },
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::TitleUpdated {
                title: "new title".to_string(),
            },
        ),
        StoryEvent::new(
            meta(),
            id,
            StoryEventKind::BodyUpdated {
                body: "new body".to_string(),
            },
        ),
    ]
}

#[test]
fn every_story_event_variant_round_trips_through_serde() {
    let variants = every_variant();
    let names: std::collections::BTreeSet<&'static str> =
        variants.iter().map(variant_name).collect();
    assert_eq!(
        names.len(),
        22,
        "the exhaustive fixture set covers every variant exactly once — a new variant must be added here (variant_name's total match will force it at compile time)"
    );
    for event in variants {
        let raw = serde_json::to_string(&event).expect("variant serializes");
        let back: StoryEvent = serde_json::from_str(&raw).expect("variant deserializes");
        assert_eq!(
            back,
            event,
            "value-level round-trip for {}",
            variant_name(&event)
        );
    }
}
