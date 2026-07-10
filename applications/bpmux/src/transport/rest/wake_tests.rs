#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use axum::Json;
use axum::extract::Extension;
use axum::response::IntoResponse;

use crate::store::SeededStore;
use crate::store::commitments::matching_users;

use super::{WakeOnEventRequest, wake_on_event};

fn store_with_two_devs_sharing_the_display() -> SeededStore {
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());
    seeded.dispatch(
        crate::store::routing::RoutingAction::Registered {
            user_id: "id-dev-1".to_string(),
            display: "dev".to_string(),
            role: None,
        }
        .into(),
    );
    seeded.dispatch(
        crate::store::routing::RoutingAction::Registered {
            user_id: "id-dev-2".to_string(),
            display: "dev".to_string(),
            role: None,
        }
        .into(),
    );
    seeded
}

async fn post_wake(seeded: &SeededStore, payload: WakeOnEventRequest) {
    let _ = wake_on_event(
        Extension(seeded.dispatcher()),
        Extension(seeded.select()),
        Json(payload),
    )
    .await
    .into_response();
}

#[tokio::test]
async fn a_body_carrying_user_id_commits_exactly_that_id() {
    let seeded = store_with_two_devs_sharing_the_display();

    post_wake(
        &seeded,
        WakeOnEventRequest {
            role: "dev".to_string(),
            user_id: Some("id-dev-1".to_string()),
            event_class: "ci_state_changed".to_string(),
            filter: None,
        },
    )
    .await;

    let payload = serde_json::json!({ "conclusion": "green" });
    let woken = seeded
        .select()
        .select(|s| matching_users(&s.commitments, "ci_state_changed", &payload));
    assert_eq!(
        woken,
        vec!["id-dev-1".to_string()],
        "a body carrying user_id commits exactly that id — id-dev-2, sharing the same display, is not armed"
    );
}

#[tokio::test]
async fn a_body_without_user_id_falls_back_to_resolving_the_role_and_commits_every_id_sharing_it() {
    let seeded = store_with_two_devs_sharing_the_display();

    post_wake(
        &seeded,
        WakeOnEventRequest {
            role: "dev".to_string(),
            user_id: None,
            event_class: "ci_state_changed".to_string(),
            filter: None,
        },
    )
    .await;

    let payload = serde_json::json!({ "conclusion": "green" });
    let mut woken = seeded
        .select()
        .select(|s| matching_users(&s.commitments, "ci_state_changed", &payload));
    woken.sort();
    assert_eq!(
        woken,
        vec!["id-dev-1".to_string(), "id-dev-2".to_string()],
        "the skew-only fallback (an old binary posting no user_id) preserves the old observable semantics: every id currently sharing the display commits and wakes"
    );
}

#[tokio::test]
async fn a_body_without_user_id_for_an_unregistered_role_commits_nothing() {
    let seeded = SeededStore::new(Vec::new(), Vec::new(), Vec::new());

    post_wake(
        &seeded,
        WakeOnEventRequest {
            role: "nobody-registered".to_string(),
            user_id: None,
            event_class: "ci_state_changed".to_string(),
            filter: None,
        },
    )
    .await;

    let payload = serde_json::json!({ "conclusion": "green" });
    let woken = seeded
        .select()
        .select(|s| matching_users(&s.commitments, "ci_state_changed", &payload));
    assert!(
        woken.is_empty(),
        "an id-keyed store cannot arm a commitment for a role with zero resolved ids — the skew-only fallback narrows here versus the old name-keyed behavior (disclosed: retires with B-5)"
    );
}
