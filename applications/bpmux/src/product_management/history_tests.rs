#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::history::{SharedHistory, react_to_story};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Story, StoryId, Verdict, Work};

fn meta(actor: &str, hour: u32) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

fn verdict_cast(story: u64, actor: &str, hour: u32, step: &str) -> StoryEvent {
    StoryEvent::new(
        meta(actor, hour),
        StoryId::new(story),
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed { step: step.into() },
            payload: Some("evidence".to_string()),
        },
    )
}

#[test]
fn a_cast_verdict_appends_a_record_with_verbatim_ts_and_actor() {
    let state = SharedHistory::default();
    let next = react_to_story(
        &state,
        &StoryAction::Event(verdict_cast(700, "owner", 9, "acceptance")),
    )
    .expect("a gate verdict is an observable signal — it must append");
    let records = next.records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].story, StoryId::new(700));
    assert_eq!(
        records[0].verdict,
        Verdict::Succeed {
            step: "acceptance".into()
        }
    );
    assert_eq!(
        records[0].ts,
        Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
        "the record's ts is the event meta's ts verbatim — never a synthesized clock read"
    );
    assert_eq!(
        records[0].actor, "owner",
        "the record's actor is the event meta's actor verbatim"
    );
}

#[test]
fn a_batch_of_events_appends_its_verdicts_in_order() {
    let state = SharedHistory::default();
    let next = react_to_story(
        &state,
        &StoryAction::Events(vec![
            verdict_cast(700, "dev", 8, "ready"),
            StoryEvent::new(
                meta("dev", 8),
                StoryId::new(700),
                StoryEventKind::StateTransitioned {
                    new_state: Work::InProgress,
                    placement: None,
                },
            ),
            verdict_cast(700, "owner", 9, "acceptance"),
        ]),
    )
    .expect("the batch carries two verdicts");
    let steps: Vec<&str> = next
        .records()
        .iter()
        .map(|r| match &r.verdict {
            Verdict::Succeed { step } | Verdict::Fail { step } => step.as_str(),
        })
        .collect();
    assert_eq!(
        steps,
        vec!["ready", "acceptance"],
        "records append in event order; the interleaved non-verdict event contributes nothing"
    );
}

#[test]
fn cache_fill_and_lifecycle_events_append_nothing_to_the_shared_grain() {
    let state = SharedHistory::default();
    let non_feeds = [
        StoryAction::Event(StoryEvent::new(
            meta("source-ingest", 10),
            StoryId::new(700),
            StoryEventKind::VerdictsFetched { verdicts: vec![] },
        )),
        StoryAction::Event(StoryEvent::new(
            meta("source-ingest", 10),
            StoryId::new(700),
            StoryEventKind::StoryRefreshed {
                story: Box::new(seeded_story(700)),
            },
        )),
        StoryAction::Event(StoryEvent::new(
            meta("pm", 10),
            StoryId::new(700),
            StoryEventKind::StateTransitioned {
                new_state: Work::Acceptance,
                placement: None,
            },
        )),
        StoryAction::Seeded(vec![seeded_story(700)]),
    ];
    for action in non_feeds {
        assert!(
            react_to_story(&state, &action).is_none(),
            "cache fills and non-verdict lifecycle events are not gate observations — the shared grain stays untouched and the state pointer survives"
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

#[test]
fn an_untouched_grain_keeps_the_app_state_pointer() {
    use crate::store::{AppAction, AppState, app_reduce};
    let state = AppState::default();
    let next = app_reduce(
        &state,
        &AppAction::Story(StoryAction::Seeded(vec![seeded_story(700)])),
    );
    assert!(
        std::sync::Arc::ptr_eq(&state.shared_history, &next.shared_history),
        "a seed changes stories, never history — the untouched grain keeps its Arc so persisters' pointer guards stay quiet"
    );
    assert!(std::sync::Arc::ptr_eq(
        &state.local_history,
        &next.local_history
    ));
}
