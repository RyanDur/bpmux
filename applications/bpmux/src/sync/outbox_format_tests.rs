#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use crate::product_management::epic::EpicId;
use crate::product_management::story::conflict::{ConflictKind, ConflictResolveOrigin};
use crate::product_management::story::events::{EventMeta, LabelDerivedFields, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{
    Acceptor, CastVerdict, FreeComment, Stage, Story, StoryId, StoryKind, Tag, Verdict, Work,
};
use crate::sync::{MutationId, QueuedEvent, load_outbox_from_disk};

const FIX_QUEUED_TAG_ADDED: &str = r#"{"id":0,"event":{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"label":"ready"}},"pre_state":null,"last_classification":null}"#;
const FIX_QUEUED_TRANSITIONED: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"new_state":"in-progress","placement":"desk-check"}},"pre_state":null,"last_classification":null}"#;
const FIX_QUEUED_VERDICT: &str = r#"{"id":0,"event":{"VerdictCast":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"verdict":{"Fail":{"step":"acceptance"}},"payload":"narrow viewport"}},"pre_state":null,"last_classification":null}"#;
const FIX_QUEUED_REFRESHED: &str = r#"{"id":0,"event":{"StoryRefreshed":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"story":{"id":700,"title":"fixture","state":"not-started","filed_at":"2026-07-01T00:00:00Z","filed_by":"pm","body":null,"free_comments":null,"verdicts":null,"body_etag":null,"story_type":"Feature","acceptor":null,"parent_epic":null,"tags":[],"ready_order":null,"iteration_slot":null}}},"pre_state":null,"last_classification":null}"#;
const FIX_QUEUED_WITH_PRESTATE: &str = r#"{"id":42,"event":{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"label":"ready"}},"pre_state":{"id":700,"title":"fixture","state":"not-started","filed_at":"2026-07-01T00:00:00Z","filed_by":"pm","body":null,"free_comments":null,"verdicts":null,"body_etag":null,"story_type":"Feature","acceptor":null,"parent_epic":null,"tags":[],"ready_order":null,"iteration_slot":null},"last_classification":null}"#;
const FIX_LEGACY_BARE_EVENT: &str = r#"{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"label":"ready"}}"#;

const FIX_ABSENT_PLACEMENT: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"new_state":"in-progress"}},"pre_state":null,"last_classification":null}"#;
const FIX_ABSENT_PLACEMENT_NORMALIZED: &str = r#"{"id":0,"event":{"StateTransitioned":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"new_state":"in-progress","placement":null}},"pre_state":null,"last_classification":null}"#;
const FIX_ABSENT_ENVELOPE_DEFAULTS: &str = r#"{"event":{"TagAdded":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"label":"ready"}}}"#;
const FIX_ABSENT_ETAG: &str = r#"{"id":0,"event":{"BodyFetched":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"body":"hydrated"}},"pre_state":null,"last_classification":null}"#;
const FIX_ABSENT_ETAG_NORMALIZED: &str = r#"{"id":0,"event":{"BodyFetched":{"meta":{"actor":"dev","ts":"2026-07-07T12:00:00Z","peer_id":"local"},"id":700,"body":"hydrated","etag":null}},"pre_state":null,"last_classification":null}"#;

fn fixture_meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
        peer_id: PeerId::local(),
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
    StoryEvent::TagAdded {
        meta: fixture_meta(),
        id: StoryId::new(700),
        label: "ready".to_string(),
    }
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
fn queued_row_with_pre_state_round_trips_byte_identically() {
    assert_byte_round_trip(FIX_QUEUED_WITH_PRESTATE);
}

#[test]
fn legacy_bare_event_round_trips_byte_identically() {
    let parsed: StoryEvent =
        serde_json::from_str(FIX_LEGACY_BARE_EVENT).expect("bare event deserializes");
    assert_eq!(parsed, fixture_tag_added());
    let reserialized = serde_json::to_string(&parsed).expect("bare event reserializes");
    assert_eq!(reserialized, FIX_LEGACY_BARE_EVENT);
}

#[test]
fn a_row_missing_the_placement_key_defaults_and_normalizes_on_rewrite() {
    let parsed: QueuedEvent =
        serde_json::from_str(FIX_ABSENT_PLACEMENT).expect("pre-placement row deserializes");
    match &parsed.event {
        StoryEvent::StateTransitioned { placement, .. } => assert_eq!(placement, &None),
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
    match &parsed.event {
        StoryEvent::BodyFetched { etag, .. } => assert_eq!(etag, &None),
        other => panic!("fixture is a BodyFetched row; got {other:?}"),
    }
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FIX_ABSENT_ETAG_NORMALIZED,
        "rewriting a pre-etag row lands the defaulted key explicitly — the documented normalization, not byte-preservation"
    );
}

#[test]
fn a_row_missing_every_envelope_field_defaults_and_normalizes_on_rewrite() {
    let parsed: QueuedEvent = serde_json::from_str(FIX_ABSENT_ENVELOPE_DEFAULTS)
        .expect("pre-envelope-fields row deserializes");
    assert_eq!(parsed.id, MutationId::default());
    assert_eq!(parsed.event, fixture_tag_added());
    assert!(parsed.pre_state.is_none());
    assert!(parsed.last_classification.is_none());
    assert_eq!(
        serde_json::to_string(&parsed).unwrap(),
        FIX_QUEUED_TAG_ADDED,
        "rewriting a pre-envelope-fields row lands the defaulted fields explicitly, converging on the current full shape"
    );
}

#[test]
fn a_legacy_bare_event_outbox_file_loads_through_the_real_loader() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("board-outbox.json");
    std::fs::write(&path, format!("[{FIX_LEGACY_BARE_EVENT}]")).unwrap();
    let loaded = load_outbox_from_disk(&path).expect("legacy file loads");
    assert_eq!(loaded.len(), 1);
    let entry = loaded.front().expect("one entry");
    assert_eq!(entry.id, MutationId::default());
    assert_eq!(entry.event, fixture_tag_added());
    assert!(entry.pre_state.is_none());
    assert!(entry.last_classification.is_none());
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
    match event {
        StoryEvent::StoryCreated { .. } => "StoryCreated",
        StoryEvent::StateTransitioned { .. } => "StateTransitioned",
        StoryEvent::TagAdded { .. } => "TagAdded",
        StoryEvent::TagRemoved { .. } => "TagRemoved",
        StoryEvent::TagSet { .. } => "TagSet",
        StoryEvent::TagUnset { .. } => "TagUnset",
        StoryEvent::IterationSlotSet { .. } => "IterationSlotSet",
        StoryEvent::IterationSlotCleared { .. } => "IterationSlotCleared",
        StoryEvent::VerdictCast { .. } => "VerdictCast",
        StoryEvent::FreeCommentAppended { .. } => "FreeCommentAppended",
        StoryEvent::BodyFetched { .. } => "BodyFetched",
        StoryEvent::FreeCommentsFetched { .. } => "FreeCommentsFetched",
        StoryEvent::VerdictsFetched { .. } => "VerdictsFetched",
        StoryEvent::StoryClosed { .. } => "StoryClosed",
        StoryEvent::StoryReopened { .. } => "StoryReopened",
        StoryEvent::AcceptorAssigned { .. } => "AcceptorAssigned",
        StoryEvent::ParentEpicSet { .. } => "ParentEpicSet",
        StoryEvent::ParentEpicUnset { .. } => "ParentEpicUnset",
        StoryEvent::CurrentIterationDesignated { .. } => "CurrentIterationDesignated",
        StoryEvent::CurrentIterationCleared { .. } => "CurrentIterationCleared",
        StoryEvent::StoryRefreshed { .. } => "StoryRefreshed",
        StoryEvent::LabelsRefreshed { .. } => "LabelsRefreshed",
        StoryEvent::TitleUpdated { .. } => "TitleUpdated",
        StoryEvent::BodyUpdated { .. } => "BodyUpdated",
        StoryEvent::ConflictDetected { .. } => "ConflictDetected",
        StoryEvent::ConflictResolved { .. } => "ConflictResolved",
    }
}

fn every_variant() -> Vec<StoryEvent> {
    let meta = fixture_meta;
    let id = StoryId::new(700);
    vec![
        StoryEvent::StoryCreated {
            meta: meta(),
            id,
            title: "t".to_string(),
            body: "b".to_string(),
            initial_state: Work::NotStarted,
            story_type: StoryKind::Feature,
        },
        StoryEvent::StateTransitioned {
            meta: meta(),
            id,
            new_state: Work::InProgress,
            placement: Some(Stage::DeskCheck.id()),
        },
        StoryEvent::TagAdded {
            meta: meta(),
            id,
            label: "ready".to_string(),
        },
        StoryEvent::TagRemoved {
            meta: meta(),
            id,
            label: "ready".to_string(),
        },
        StoryEvent::TagSet {
            meta: meta(),
            id,
            tag: Tag::DeskCheckFail,
        },
        StoryEvent::TagUnset {
            meta: meta(),
            id,
            tag: Tag::DeskCheckFail,
        },
        StoryEvent::IterationSlotSet {
            meta: meta(),
            id,
            name: "Pegasus".to_string(),
            position: 100,
        },
        StoryEvent::IterationSlotCleared { meta: meta(), id },
        StoryEvent::VerdictCast {
            meta: meta(),
            id,
            verdict: Verdict::Fail {
                step: "acceptance".into(),
            },
            payload: Some("reason".to_string()),
        },
        StoryEvent::FreeCommentAppended {
            meta: meta(),
            id,
            body: "comment".to_string(),
        },
        StoryEvent::BodyFetched {
            meta: meta(),
            id,
            body: "body".to_string(),
            etag: Some("W/\"etag\"".to_string()),
        },
        StoryEvent::FreeCommentsFetched {
            meta: meta(),
            id,
            comments: vec![FreeComment {
                body: "c".to_string(),
                author: "pm".to_string(),
                ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
            }],
        },
        StoryEvent::VerdictsFetched {
            meta: meta(),
            id,
            verdicts: vec![CastVerdict {
                verdict: Verdict::Succeed {
                    step: "in-progress".into(),
                },
                ts: Utc.with_ymd_and_hms(2026, 7, 7, 12, 0, 0).unwrap(),
                actor: "dev".to_string(),
                payload: None,
            }],
        },
        StoryEvent::StoryClosed {
            meta: meta(),
            id,
            reason: Some("done".to_string()),
        },
        StoryEvent::StoryReopened { meta: meta(), id },
        StoryEvent::AcceptorAssigned {
            meta: meta(),
            id,
            acceptor: Acceptor::Team,
        },
        StoryEvent::ParentEpicSet {
            meta: meta(),
            id,
            epic_id: EpicId::new(9),
        },
        StoryEvent::ParentEpicUnset { meta: meta(), id },
        StoryEvent::CurrentIterationDesignated { meta: meta(), id },
        StoryEvent::CurrentIterationCleared { meta: meta(), id },
        StoryEvent::StoryRefreshed {
            meta: meta(),
            id,
            story: Box::new(fixture_story()),
        },
        StoryEvent::LabelsRefreshed {
            meta: meta(),
            id,
            fields: LabelDerivedFields {
                raw_labels: vec!["feature".to_string()],
                acceptor: Some(Acceptor::Team),
                parent_epic: Some(EpicId::new(9)),
                tags: vec![Tag::DeskCheckFail],
                ready_order: Some(100),
                iteration_slot: None,
            },
        },
        StoryEvent::TitleUpdated {
            meta: meta(),
            id,
            title: "new title".to_string(),
        },
        StoryEvent::BodyUpdated {
            meta: meta(),
            id,
            body: "new body".to_string(),
        },
        StoryEvent::ConflictDetected {
            meta: meta(),
            id,
            kind: ConflictKind::DivergentState,
        },
        StoryEvent::ConflictResolved {
            meta: meta(),
            id,
            origin: ConflictResolveOrigin::OperatorAction,
        },
    ]
}

#[test]
fn every_story_event_variant_round_trips_through_serde() {
    let variants = every_variant();
    let names: std::collections::BTreeSet<&'static str> =
        variants.iter().map(variant_name).collect();
    assert_eq!(
        names.len(),
        26,
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
