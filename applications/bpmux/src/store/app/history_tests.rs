#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};
use tempfile::TempDir;
use tokio::sync::broadcast;

use super::history::{LocalHistoryPersister, SharedHistoryPersister};
use crate::board::history_persistence as local_persistence;
use crate::hub::Hub;
use crate::kernel::{Dispatch, Listener, Reaction};
use crate::message::Message;
use crate::product_management::history_persistence as shared_persistence;
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, Story, StoryId, Verdict, Work};
use crate::store::{AppAction, AppState, app_reduce};

fn fresh_hub() -> (Arc<Hub>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    (hub, dir)
}

fn no_op_dispatch() -> Dispatch<AppAction> {
    Arc::new(|_action: AppAction| {})
}

fn verdict_action() -> AppAction {
    AppAction::Story(StoryAction::Event(StoryEvent::new(
        EventMeta {
            actor: "owner".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
        },
        StoryId::new(700),
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed {
                step: "acceptance".into(),
            },
            payload: None,
        },
    )))
}

fn transition_action() -> AppAction {
    AppAction::Story(StoryAction::Event(StoryEvent::new(
        EventMeta {
            actor: "dev".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 7, 7, 9, 0, 0).unwrap(),
        },
        StoryId::new(700),
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            placement: Some(Stage::DeskCheck.id()),
        },
    )))
}

fn seed_action() -> AppAction {
    AppAction::Story(StoryAction::Seeded(vec![Story::new_minimal(
        StoryId::new(700),
        "fixture".to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]))
}

async fn drive(
    listener: &dyn Listener<AppState, AppAction>,
    previous: AppState,
    action: &AppAction,
) {
    let current = app_reduce(&previous, action);
    let reaction = Reaction {
        action: action.clone(),
        previous: Arc::new(previous),
        current: Arc::new(current),
        dispatch: no_op_dispatch(),
    };
    listener.on_action(reaction).await;
}

#[tokio::test]
async fn a_shared_history_change_is_flushed_to_disk() {
    let (hub, _dir) = fresh_hub();
    let listener = SharedHistoryPersister::new(hub.clone());
    drive(&listener, AppState::default(), &verdict_action()).await;
    assert_eq!(
        shared_persistence::load(&hub.dir).records().len(),
        1,
        "a verdict append flushes the shared grain behind the pointer guard"
    );
}

#[tokio::test]
async fn an_untouched_shared_grain_writes_no_file() {
    let (hub, _dir) = fresh_hub();
    let listener = SharedHistoryPersister::new(hub.clone());
    drive(&listener, AppState::default(), &seed_action()).await;
    assert!(
        !shared_persistence::shared_history_path(&hub.dir).exists(),
        "the pointer guard keeps the persister quiet when the shared grain is untouched"
    );
}

#[tokio::test]
async fn a_local_history_change_is_flushed_to_disk() {
    let (hub, _dir) = fresh_hub();
    let listener = LocalHistoryPersister::new(hub.clone());
    drive(&listener, AppState::default(), &transition_action()).await;
    assert_eq!(
        local_persistence::load(&hub.dir).records().len(),
        1,
        "a placement move flushes the local grain behind the pointer guard"
    );
}

#[tokio::test]
async fn an_untouched_local_grain_writes_no_file() {
    let (hub, _dir) = fresh_hub();
    let listener = LocalHistoryPersister::new(hub.clone());
    drive(&listener, AppState::default(), &seed_action()).await;
    assert!(
        !local_persistence::placement_history_path(&hub.dir).exists(),
        "the pointer guard keeps the persister quiet when the local grain is untouched"
    );
}

#[test]
fn a_cold_boot_hydrates_both_history_grains_from_their_files() {
    let (hub, _dir) = fresh_hub();
    let shared = app_reduce(&AppState::default(), &verdict_action());
    let local = app_reduce(&AppState::default(), &transition_action());
    shared_persistence::save(&hub.dir, &shared.shared_history).unwrap();
    local_persistence::save(&hub.dir, &local.local_history).unwrap();

    let state = AppState::with_board(crate::board::persistence::load(&hub.dir))
        .with_shared_history(shared_persistence::load(&hub.dir))
        .with_local_history(local_persistence::load(&hub.dir));

    assert_eq!(
        state.shared_history.records(),
        shared.shared_history.records(),
        "a cold boot restores the shared grain from shared-history.json"
    );
    assert_eq!(
        state.local_history.records(),
        local.local_history.records(),
        "a cold boot restores the local grain from placement-history.json"
    );
}
