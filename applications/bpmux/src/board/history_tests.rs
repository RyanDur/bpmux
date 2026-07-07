#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::history::{PlacementHistory, react_to_story};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, Story, StoryId, Verdict, Work};

fn meta(actor: &str, hour: u32) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn transition(
    story: u64,
    actor: &str,
    hour: u32,
    to: Work,
    placement: Option<Stage>,
) -> StoryEvent {
    StoryEvent::new(
        meta(actor, hour),
        StoryId::new(story),
        StoryEventKind::StateTransitioned {
            new_state: to,
            placement: placement.map(|s| s.id()),
        },
    )
}

#[test]
fn a_state_transition_appends_a_placement_record_with_verbatim_ts_and_actor() {
    let state = PlacementHistory::default();
    let next = react_to_story(
        &state,
        &StoryAction::Event(transition(
            700,
            "dev",
            9,
            Work::InProgress,
            Some(Stage::DeskCheck),
        )),
    )
    .expect("a placement-bearing transition is an observable move — it must append");
    let records = next.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].story, StoryId::new(700));
    assert_eq!(records[0].placement, Some(Stage::DeskCheck.id()));
    assert_eq!(records[0].to, Work::InProgress);
    assert_eq!(
        records[0].ts,
        Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
        "the record's ts is the event meta's ts verbatim"
    );
    assert_eq!(records[0].actor, "dev");
}

#[test]
fn a_transition_without_a_placement_records_the_departure_from_the_middle() {
    let state = PlacementHistory::default();
    let next = react_to_story(
        &state,
        &StoryAction::Event(transition(700, "designer", 10, Work::Acceptance, None)),
    )
    .expect("leaving the middle is itself a move the timings need");
    let records = next.records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].placement, None,
        "placement None records the departure so sub-section spans can close"
    );
    assert_eq!(records[0].to, Work::Acceptance);
}

#[test]
fn non_transition_events_append_nothing_to_the_local_grain() {
    let state = PlacementHistory::default();
    let non_feeds = [
        StoryAction::Event(StoryEvent::new(
            meta("owner", 9),
            StoryId::new(700),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Succeed {
                    step: "acceptance".into(),
                },
                payload: None,
            },
        )),
        StoryAction::Event(StoryEvent::new(
            meta("source-ingest", 10),
            StoryId::new(700),
            StoryEventKind::StoryRefreshed {
                story: Box::new(seeded_story(700)),
            },
        )),
        StoryAction::Seeded(vec![seeded_story(700)]),
    ];
    for action in non_feeds {
        assert!(
            react_to_story(&state, &action).is_none(),
            "only real transitions carry placement observations — the local grain stays untouched"
        );
    }
}

fn seeded_story(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        "fixture".to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}
