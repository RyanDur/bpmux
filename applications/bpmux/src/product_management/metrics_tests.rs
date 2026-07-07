#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::num::NonZeroUsize;

use chrono::{DateTime, TimeZone, Utc};

use super::history::{SharedHistory, react_to_story};
use super::iteration::{Iteration, IterationId};
use super::metrics::{
    acceptance_rejection_rate, accepted, lead_time_median_hours, velocity, velocity_series,
};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{StoryId, Verdict};

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap()
}

fn verdict_event(story: u64, hour: u32, verdict: Verdict) -> StoryEvent {
    StoryEvent::new(
        EventMeta {
            actor: "owner".to_string(),
            ts: at(hour),
            peer_id: PeerId::local(),
        },
        StoryId::new(story),
        StoryEventKind::VerdictCast {
            verdict,
            payload: None,
        },
    )
}

fn ready(story: u64, hour: u32) -> StoryEvent {
    verdict_event(
        story,
        hour,
        Verdict::Succeed {
            step: "ready".into(),
        },
    )
}

fn accept(story: u64, hour: u32) -> StoryEvent {
    verdict_event(
        story,
        hour,
        Verdict::Succeed {
            step: "acceptance".into(),
        },
    )
}

fn reject(story: u64, hour: u32) -> StoryEvent {
    verdict_event(
        story,
        hour,
        Verdict::Fail {
            step: "acceptance".into(),
        },
    )
}

fn history(events: Vec<StoryEvent>) -> SharedHistory {
    react_to_story(&SharedHistory::default(), &StoryAction::Events(events)).unwrap_or_default()
}

fn iteration(id: u64, order: u32, complete: bool, stories: &[u64]) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: at(0),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: Some(order),
        stories: stories.iter().map(|s| StoryId::new(*s)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: false,
        complete,
    }
}

fn features(ids: &[u64]) -> impl Fn(StoryId) -> bool + '_ {
    move |story: StoryId| ids.iter().any(|id| StoryId::new(*id) == story)
}

#[test]
fn accepted_is_true_only_when_an_acceptance_succeed_record_exists() {
    let history = history(vec![accept(1, 9), ready(2, 8), reject(3, 9)]);
    assert!(accepted(&history, StoryId::new(1)));
    assert!(
        !accepted(&history, StoryId::new(2)),
        "a pickup ready verdict is not an acceptance"
    );
    assert!(
        !accepted(&history, StoryId::new(3)),
        "a failed acceptance ceremony is not an acceptance"
    );
    assert!(!accepted(&history, StoryId::new(4)));
}

#[test]
fn velocity_counts_only_accepted_features_each_worth_one() {
    let history = history(vec![accept(1, 9), accept(2, 9), accept(3, 9)]);
    let is_feature = features(&[1, 2, 4]);
    let iterations = [iteration(10, 1, true, &[1, 2, 3, 4])];
    assert_eq!(
        velocity_series(&history, &iterations, &is_feature),
        vec![(IterationId::new(10), 2)],
        "the accepted chore (3) and the unaccepted feature (4) contribute nothing"
    );
    assert_eq!(
        velocity(&history, &iterations, &is_feature, NonZeroUsize::MIN),
        Some(2.0)
    );
}

#[test]
fn velocity_rolling_window_slides_to_the_last_n_complete_iterations() {
    let history = history(vec![
        accept(1, 9),
        accept(2, 9),
        accept(3, 9),
        accept(4, 9),
        accept(5, 9),
        accept(6, 9),
    ]);
    let is_feature = features(&[1, 2, 3, 4, 5, 6]);
    let iterations = [
        iteration(10, 1, true, &[1, 2]),
        iteration(11, 2, true, &[3, 4, 5, 6]),
    ];
    assert_eq!(
        velocity_series(&history, &iterations, &is_feature),
        vec![(IterationId::new(10), 2), (IterationId::new(11), 4)]
    );
    assert_eq!(
        velocity(&history, &iterations, &is_feature, NonZeroUsize::MIN),
        Some(4.0),
        "a window of one reports the most recent complete iteration's count"
    );
    assert_eq!(
        velocity(
            &history,
            &iterations,
            &is_feature,
            NonZeroUsize::new(2).unwrap()
        ),
        Some(3.0),
        "a window of two averages the two counts"
    );
}

#[test]
fn velocity_excludes_incomplete_iterations() {
    let history = history(vec![accept(1, 9), accept(2, 9), accept(3, 9)]);
    let is_feature = features(&[1, 2, 3]);
    let iterations = [
        iteration(10, 1, true, &[1, 2]),
        iteration(11, 2, false, &[3]),
    ];
    assert_eq!(
        velocity_series(&history, &iterations, &is_feature),
        vec![(IterationId::new(10), 2)],
        "the current/incomplete iteration is not yet a data point"
    );
    assert_eq!(
        velocity(&history, &iterations, &is_feature, NonZeroUsize::MIN),
        Some(2.0)
    );
}

#[test]
fn velocity_is_none_when_no_iteration_is_complete() {
    let history = history(vec![accept(1, 9)]);
    let is_feature = features(&[1]);
    let iterations = [iteration(11, 1, false, &[1])];
    assert_eq!(
        velocity(&history, &iterations, &is_feature, NonZeroUsize::MIN),
        None
    );
    assert_eq!(
        velocity(&history, &[], &is_feature, NonZeroUsize::MIN),
        None
    );
}

#[test]
fn lead_time_measures_first_pickup_to_accept_across_a_rework_loop() {
    let history = history(vec![ready(1, 1), reject(1, 2), ready(1, 3), accept(1, 4)]);
    assert_eq!(
        lead_time_median_hours(&history),
        Some(3.0),
        "lead time spans the first ready (hour 1) to accept (hour 4), not the re-pickup at hour 3"
    );
}

#[test]
fn lead_time_ignores_a_story_accepted_without_a_ready_record() {
    let history = history(vec![accept(1, 4)]);
    assert_eq!(
        lead_time_median_hours(&history),
        None,
        "a thin pre-history story with no pickup confabulates no start"
    );
}

#[test]
fn lead_time_is_the_median_over_accepted_stories() {
    let history = history(vec![ready(1, 1), accept(1, 4), ready(2, 1), accept(2, 3)]);
    assert_eq!(
        lead_time_median_hours(&history),
        Some(2.5),
        "median of a 3-hour and a 2-hour lead"
    );
}

#[test]
fn lead_time_is_independent_of_record_vector_order() {
    let ordered = history(vec![ready(1, 1), accept(1, 4)]);
    let scrambled = history(vec![accept(1, 4), ready(1, 1)]);
    assert_eq!(
        lead_time_median_hours(&scrambled),
        lead_time_median_hours(&ordered),
        "time math keys on ts, never on append order"
    );
    assert_eq!(lead_time_median_hours(&scrambled), Some(3.0));
}

#[test]
fn acceptance_rejection_rate_is_fails_over_all_acceptance_ceremonies() {
    let history = history(vec![reject(1, 9), accept(2, 9), accept(3, 9), accept(4, 9)]);
    assert_eq!(acceptance_rejection_rate(&history), Some(0.25));
}

#[test]
fn acceptance_rejection_rate_is_none_without_any_acceptance_ceremony() {
    let history = history(vec![ready(1, 8)]);
    assert_eq!(acceptance_rejection_rate(&history), None);
}
