#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use super::reactions::ReactionsPersister;
use crate::hub::Hub;
use crate::kernel::{Dispatch, Listener, Reaction};
use crate::message::Message;
use crate::store::reactions::{aggregated_for_channel, from_store, to_store};
use crate::store::{AppAction, AppState, Presence, ReactionsAction, RosterAction, app_reduce};

fn fresh_hub() -> (Arc<Hub>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    (hub, dir)
}

fn no_op_dispatch() -> Dispatch<AppAction> {
    Arc::new(|_action: AppAction| {})
}

fn toggle_action(user: &str) -> AppAction {
    AppAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: user.to_string(),
        ts_iso: "2026-07-08T00:00:00Z".to_string(),
    })
}

fn roster_action() -> AppAction {
    AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-dev".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    })
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
async fn a_reaction_toggle_is_flushed_to_disk() {
    let (hub, _dir) = fresh_hub();
    let listener = ReactionsPersister::new(hub.clone());
    drive(&listener, AppState::default(), &toggle_action("owner")).await;
    let hydrated = from_store(&crate::reactions::load(&hub.dir));
    assert_eq!(
        aggregated_for_channel(&hydrated, "broadcast")[0].count,
        1,
        "a toggle flushes the reactions grain behind the pointer guard"
    );
}

#[tokio::test]
async fn an_untouched_reactions_grain_writes_no_file() {
    let (hub, _dir) = fresh_hub();
    let listener = ReactionsPersister::new(hub.clone());
    drive(&listener, AppState::default(), &roster_action()).await;
    assert!(
        !crate::reactions::reactions_path(&hub.dir).exists(),
        "the pointer guard keeps the persister quiet when reactions are untouched"
    );
}

#[test]
fn a_cold_boot_hydrates_the_reactions_grain_from_its_file() {
    let (hub, _dir) = fresh_hub();
    let seeded = app_reduce(&AppState::default(), &toggle_action("owner"));
    crate::reactions::save(&hub.dir, &to_store(&seeded.reactions)).unwrap();

    let state = AppState::default().with_reactions(from_store(&crate::reactions::load(&hub.dir)));

    assert_eq!(
        aggregated_for_channel(&state.reactions, "broadcast"),
        aggregated_for_channel(&seeded.reactions, "broadcast"),
        "a cold boot restores the reactions grain from reactions.json"
    );
}
