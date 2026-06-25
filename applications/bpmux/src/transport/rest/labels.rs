use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{broadcast_board_invalidate, broadcast_board_snapshot, err_body, json_headers};
use crate::session;

#[derive(Deserialize)]
pub struct LabelMutationPayload {
    #[serde(default)]
    pub add: Vec<String>,
    #[serde(default)]
    pub remove: Vec<String>,
}

enum LabelOp {
    Add(String),
    Remove(String),
}

fn reorder_label_ops(payload: &LabelMutationPayload) -> Vec<LabelOp> {
    payload
        .add
        .iter()
        .cloned()
        .map(LabelOp::Add)
        .chain(payload.remove.iter().cloned().map(LabelOp::Remove))
        .collect()
}

pub async fn post_story_labels(
    State(session): State<Arc<session::Session>>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<LabelMutationPayload>,
) -> impl IntoResponse {
    let Some(refresh) = test_refresh.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    for op in reorder_label_ops(&payload) {
        let meta = crate::product_management::board::orchestrator::cache_fill_meta();
        let (event, descriptor) = match op {
            LabelOp::Add(label) => {
                let descriptor = format!("TagAdded {label} on #{number}");
                (
                    crate::product_management::board::BoardEvent::TagAdded {
                        meta,
                        number,
                        label,
                    },
                    descriptor,
                )
            }
            LabelOp::Remove(label) => {
                let descriptor = format!("TagRemoved {label} on #{number}");
                (
                    crate::product_management::board::BoardEvent::TagRemoved {
                        meta,
                        number,
                        label,
                    },
                    descriptor,
                )
            }
        };
        if let Err(e) = refresh.mutate_sync(event) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("{descriptor}: {e}")),
            );
        }
    }
    broadcast_board_invalidate(&session);
    broadcast_board_snapshot(&session, &refresh);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "ok": true, "number": number }).to_string(),
    )
}

#[cfg(test)]
mod reorder_label_ops_tests {
    use super::*;
    use crate::product_management::board::adapters::github::iteration_membership_from_labels;
    use crate::product_management::board::{BoardEvent, BoardStore};
    use crate::product_management::story::{Story, StoryState};

    fn membered_fixture(number: u64, label: &str) -> Story {
        let mut story = Story::new_minimal(
            number,
            "reorder fixture".to_string(),
            StoryState::Ready,
            chrono::Utc::now(),
            "tester".to_string(),
        );
        story.raw_labels = vec![label.to_string()];
        story
    }

    fn membership_of(store: &BoardStore, number: u64) -> Option<(u64, u32)> {
        store
            .story(number)
            .and_then(|s| iteration_membership_from_labels(&s.raw_labels))
    }

    #[test]
    fn reorder_keeps_story_membered_at_every_step_of_the_label_swap() {
        let store = BoardStore::new();
        store.seed_all(vec![membered_fixture(1687, "parent-iteration-#1655.125")]);
        assert!(
            membership_of(&store, 1687).is_some(),
            "fixture must start membered"
        );

        let payload = LabelMutationPayload {
            add: vec!["parent-iteration-#1655.106".to_string()],
            remove: vec!["parent-iteration-#1655.125".to_string()],
        };

        for op in reorder_label_ops(&payload) {
            let meta = crate::product_management::board::orchestrator::cache_fill_meta();
            let event = match op {
                LabelOp::Add(label) => BoardEvent::TagAdded {
                    meta,
                    number: 1687,
                    label,
                },
                LabelOp::Remove(label) => BoardEvent::TagRemoved {
                    meta,
                    number: 1687,
                    label,
                },
            };
            store.apply(&event);
            assert!(
                membership_of(&store, 1687).is_some(),
                "a reorder must never transiently orphan the story: removing the old parent-iteration label before adding the new one strips its only membership label, so a concurrent board read drops the row from the Now tab until refresh (the #1524 disappear)"
            );
        }

        assert_eq!(
            membership_of(&store, 1687),
            Some((1655, 106)),
            "after the swap the story is membered at the new position"
        );
    }
}
