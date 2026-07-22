#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use product_management::history::{SharedHistory, SharedHistoryAction, VerdictRecord, reduce};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId, Verdict, Work};
use store::{AppAction, AppState, app_reduce};

fn record(story: u64, actor: &str, hour: u32, step: &str) -> VerdictRecord {
    VerdictRecord {
        story: StoryId::new(story),
        verdict: Verdict::Succeed { step: step.into() },
        ts: Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap(),
        actor: actor.to_string(),
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
fn re_backfilling_identical_records_leaves_the_grain_and_pointer_untouched() {
    let backfill = SharedHistoryAction::Backfilled {
        story: StoryId::new(700),
        records: vec![record(700, "owner", 9, "ready")],
    };
    let once = reduce(&SharedHistory::default(), &backfill)
        .expect("the first backfill populates the empty grain");
    assert!(
        reduce(&once, &backfill).is_none(),
        "an identical re-backfill is a no-op — same durable input, same state"
    );
    let state = AppState::default().with_shared_history(once);
    let next = app_reduce(&state, &AppAction::SharedHistory(backfill));
    assert!(
        std::sync::Arc::ptr_eq(&state.shared_history, &next.shared_history),
        "a no-op backfill keeps the Arc so the persister's pointer guard stays quiet"
    );
}

#[test]
fn an_untouched_grain_keeps_the_app_state_pointer() {
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
