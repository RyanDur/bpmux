#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

use super::view::dispatch_view_velocity;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Story, StoryId, StoryKind, Verdict, Work};
use crate::store::{AppAction, SeededStore};
use crate::workflow::StageId;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap()
}

fn meta(hour: u32) -> EventMeta {
    EventMeta {
        actor: "owner".to_string(),
        actor_id: None,
        ts: at(hour),
    }
}

fn verdict(story: u64, hour: u32, verdict: Verdict) -> StoryEvent {
    StoryEvent::new(
        meta(hour),
        StoryId::new(story),
        StoryEventKind::VerdictCast {
            verdict,
            payload: None,
        },
    )
}

fn placement(story: u64, hour: u32, stage: &str) -> StoryEvent {
    StoryEvent::new(
        meta(hour),
        StoryId::new(story),
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            placement: Some(StageId::from(stage)),
        },
    )
}

fn feature(id: u64) -> Story {
    let mut story = Story::new_minimal(
        StoryId::new(id),
        format!("feature-{id}"),
        Work::InProgress,
        at(0),
        "dev".to_string(),
    );
    story.story_type = StoryKind::Feature;
    story
}

fn complete_iteration(id: u64, stories: &[u64]) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: at(0),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: Some(1),
        stories: stories.iter().map(|s| StoryId::new(*s)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: false,
        complete: true,
    }
}

#[test]
fn selector_fed_metrics_report_tracker_velocity_lead_time_and_rework_from_the_store() {
    let store = SeededStore::new(vec![], vec![complete_iteration(10, &[1])], vec![]);
    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![feature(1)])));
    store.dispatch(AppAction::Story(StoryAction::Events(vec![
        verdict(
            1,
            1,
            Verdict::Succeed {
                step: "ready".into(),
            },
        ),
        verdict(
            1,
            4,
            Verdict::Succeed {
                step: "acceptance".into(),
            },
        ),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Events(vec![
        placement(1, 0, "review"),
        placement(1, 1, "build"),
        placement(1, 2, "review"),
    ])));

    let json: Value =
        serde_json::from_str(&dispatch_view_velocity(&store.select()).unwrap()).unwrap();

    assert_eq!(
        json["velocity"], 1.0,
        "one accepted feature in the last complete iteration, window 1"
    );
    assert_eq!(json["velocity_window_iterations"], 1);
    let iterations = json["iterations"].as_array().unwrap();
    assert_eq!(iterations.len(), 1);
    assert_eq!(iterations[0]["iteration"], 10);
    assert_eq!(iterations[0]["accepted_features"], 1);
    assert_eq!(
        json["lead_time_median_hours"], 3.0,
        "ready hour 1 to accept hour 4"
    );
    assert_eq!(
        json["acceptance_rejection_rate"], 0.0,
        "one accept, no rejection"
    );
    let timings = json["sub_section_timings"].as_array().unwrap();
    let review = timings
        .iter()
        .find(|t| t["position"] == serde_json::json!({"kind": "stage", "id": "review"}))
        .expect("review sub-section timing present with self-describing tagged position");
    assert_eq!(review["median_hours"], 1.0);
    assert_eq!(
        json["rework"]["total"], 1,
        "review is departed then re-entered once"
    );
    assert_eq!(
        json["rework"]["per_story"][0],
        serde_json::json!({"story": 1, "reentries": 1})
    );
}

#[test]
fn selector_fed_metrics_speak_nulls_for_a_store_with_no_history() {
    let store = SeededStore::new(vec![], vec![], vec![]);

    let json: Value =
        serde_json::from_str(&dispatch_view_velocity(&store.select()).unwrap()).unwrap();

    assert_eq!(
        json["velocity"],
        Value::Null,
        "no complete iteration is a null velocity, not a zero"
    );
    assert_eq!(json["velocity_window_iterations"], 1);
    assert_eq!(json["iterations"], serde_json::json!([]));
    assert_eq!(json["lead_time_median_hours"], Value::Null);
    assert_eq!(json["acceptance_rejection_rate"], Value::Null);
    assert_eq!(json["sub_section_timings"], serde_json::json!([]));
    assert_eq!(
        json["rework"],
        serde_json::json!({"total": 0, "per_story": []})
    );
}
