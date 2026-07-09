#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;

use super::routing::RoutingPersister;
use crate::kernel::{Dispatch, Listener, Reaction};
use crate::routing::UserRepo;
use crate::store::routing::{RoutingAction, from_users};
use crate::store::{AppAction, AppState, app_reduce};

fn no_op_dispatch() -> Dispatch<AppAction> {
    Arc::new(|_action: AppAction| {})
}

fn repo(dir: &TempDir) -> Arc<dyn UserRepo> {
    crate::storage::sqlite_test_storage(dir.path())
}

async fn drive(
    listener: &RoutingPersister,
    previous: AppState,
    actions: &[RoutingAction],
) -> AppState {
    let mut previous = previous;
    for action in actions {
        let app_action = AppAction::Routing(action.clone());
        let current = app_reduce(&previous, &app_action);
        listener
            .on_action(Reaction {
                action: app_action,
                previous: Arc::new(previous.clone()),
                current: Arc::new(current.clone()),
                dispatch: no_op_dispatch(),
            })
            .await;
        previous = current;
    }
    previous
}

fn instant(rfc3339: &str) -> i64 {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .unwrap()
        .timestamp_millis()
}

#[tokio::test]
async fn a_register_then_cursor_advance_round_trips_through_the_users_table_into_a_hydrated_slice()
{
    let dir = TempDir::new().unwrap();
    let repo = repo(&dir);
    let listener = RoutingPersister::new(repo.clone());
    drive(
        &listener,
        AppState::default(),
        &[
            RoutingAction::Registered {
                user_id: "sess-dev".to_string(),
                display: "dev:claude-code".to_string(),
            },
            RoutingAction::CursorAdvanced {
                user_id: "sess-dev".to_string(),
                cursor: "2026-06-11T20:00:05Z".to_string(),
            },
        ],
    )
    .await;

    let hydrated = from_users(&repo.load_users().unwrap());
    assert_eq!(
        crate::store::routing::resolve(&hydrated, "dev:claude-code"),
        vec!["sess-dev".to_string()],
        "the write-behind persisted the registration; a fresh hydrate resolves it identically"
    );
    assert_eq!(
        crate::store::routing::cursor(&hydrated, "sess-dev")
            .as_deref()
            .map(instant),
        Some(instant("2026-06-11T20:00:05Z")),
        "the cursor advance persisted as last_delivered and hydrates back to the same instant"
    );
}

#[tokio::test]
async fn a_burst_of_cursor_advances_coalesces_to_the_latest_monotonic_position() {
    let dir = TempDir::new().unwrap();
    let repo = repo(&dir);
    let listener = RoutingPersister::new(repo.clone());
    drive(
        &listener,
        AppState::default(),
        &[
            RoutingAction::Registered {
                user_id: "sess-dev".to_string(),
                display: "dev".to_string(),
            },
            RoutingAction::CursorAdvanced {
                user_id: "sess-dev".to_string(),
                cursor: "2026-06-11T20:00:01Z".to_string(),
            },
            RoutingAction::CursorAdvanced {
                user_id: "sess-dev".to_string(),
                cursor: "2026-06-11T20:00:09Z".to_string(),
            },
            RoutingAction::CursorAdvanced {
                user_id: "sess-dev".to_string(),
                cursor: "2026-06-11T20:00:03Z".to_string(),
            },
        ],
    )
    .await;

    let users = repo.load_users().unwrap();
    let user = users.iter().find(|u| u.user_id == "sess-dev").unwrap();
    assert_eq!(
        user.last_delivered_at,
        Some(instant("2026-06-11T20:00:09Z")),
        "a burst coalesces to the highest cursor — monotonic-max in the reducer means the write-behind persists the latest, never a backward one"
    );
}

#[tokio::test]
async fn a_rename_persists_the_new_display_so_only_it_resolves_on_reload() {
    let dir = TempDir::new().unwrap();
    let repo = repo(&dir);
    let listener = RoutingPersister::new(repo.clone());
    drive(
        &listener,
        AppState::default(),
        &[
            RoutingAction::Registered {
                user_id: "sess-1".to_string(),
                display: "dev".to_string(),
            },
            RoutingAction::Renamed {
                user_id: "sess-1".to_string(),
                display: "dev-renamed".to_string(),
            },
        ],
    )
    .await;

    let hydrated = from_users(&repo.load_users().unwrap());
    assert_eq!(
        crate::store::routing::resolve(&hydrated, "dev-renamed"),
        vec!["sess-1".to_string()],
        "the rename persisted — the new display resolves to the fixed id across a reload"
    );
    assert!(
        crate::store::routing::resolve(&hydrated, "dev").is_empty(),
        "the old display no longer resolves — the durable row moved with the rename"
    );
}

#[tokio::test]
async fn a_reap_deletes_the_unclaimed_duplicate_row_leaving_only_the_live_id() {
    let dir = TempDir::new().unwrap();
    let repo = repo(&dir);
    let listener = RoutingPersister::new(repo.clone());
    drive(
        &listener,
        AppState::default(),
        &[
            RoutingAction::Registered {
                user_id: "dev-stale".to_string(),
                display: "dev".to_string(),
            },
            RoutingAction::Registered {
                user_id: "dev-live".to_string(),
                display: "dev".to_string(),
            },
            RoutingAction::TerminalLinked {
                user_id: "dev-live".to_string(),
                terminal_id: "term-live".to_string(),
            },
            RoutingAction::Reaped {
                display: "dev".to_string(),
            },
        ],
    )
    .await;

    let ids: Vec<String> = repo
        .load_users()
        .unwrap()
        .into_iter()
        .map(|u| u.user_id)
        .collect();
    assert!(
        ids.contains(&"dev-live".to_string()),
        "the live claimed identity persists"
    );
    assert!(
        !ids.contains(&"dev-stale".to_string()),
        "the reap of the unclaimed duplicate deletes its row from the users table"
    );
}

#[tokio::test]
async fn a_non_routing_transition_writes_nothing_behind_the_pointer_guard() {
    let dir = TempDir::new().unwrap();
    let repo = repo(&dir);
    let listener = RoutingPersister::new(repo.clone());
    let previous = AppState::default();
    let current = previous.clone();
    listener
        .on_action(Reaction {
            action: AppAction::Routing(RoutingAction::Reaped {
                display: "nobody".to_string(),
            }),
            previous: Arc::new(previous),
            current: Arc::new(current),
            dispatch: no_op_dispatch(),
        })
        .await;
    assert!(
        repo.load_users().unwrap().is_empty(),
        "an unchanged routing grain (ptr-equal) writes nothing"
    );
}
