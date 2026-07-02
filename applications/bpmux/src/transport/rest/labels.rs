use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{broadcast_board_invalidate, broadcast_stories, err_body, json_headers};
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
    axum::Extension(orchestrator): axum::Extension<crate::server::OrchestratorHandle>,
    axum::Extension(dispatcher): axum::Extension<crate::store::Dispatch>,
    axum::extract::Path(id): axum::extract::Path<u64>,
    Json(payload): Json<LabelMutationPayload>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    for op in reorder_label_ops(&payload) {
        let meta = crate::product_management::board::orchestrator::cache_fill_meta();
        let story_id = crate::product_management::story::StoryId::new(id);
        let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
        let (event, descriptor) = match op {
            LabelOp::Add(label) => {
                let descriptor = format!("TagAdded {label} on #{id}");
                let event = match codec.decode_tag(&label) {
                    Some(tag) => crate::product_management::board::StoryEvent::TagSet {
                        meta,
                        id: story_id,
                        tag,
                    },
                    None => crate::product_management::board::StoryEvent::TagAdded {
                        meta,
                        id: story_id,
                        label,
                    },
                };
                (event, descriptor)
            }
            LabelOp::Remove(label) => {
                let descriptor = format!("TagRemoved {label} on #{id}");
                let event = match codec.decode_tag(&label) {
                    Some(tag) => crate::product_management::board::StoryEvent::TagUnset {
                        meta,
                        id: story_id,
                        tag,
                    },
                    None => crate::product_management::board::StoryEvent::TagRemoved {
                        meta,
                        id: story_id,
                        label,
                    },
                };
                (event, descriptor)
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
    if refresh
        .iteration(crate::product_management::iteration::IterationId::new(id))
        .is_some()
        && let Some(story) = refresh.story(id)
    {
        dispatcher.dispatch(crate::store::AppAction::Iteration(
            crate::product_management::iteration::IterationAction::Upserted(vec![
                crate::product_management::board::adapters::github::decode_iteration_from_story(
                    &story,
                ),
            ]),
        ));
    }
    broadcast_board_invalidate(&session);
    broadcast_stories(&session, &refresh);
    (
        StatusCode::OK,
        json_headers(),
        serde_json::json!({ "ok": true, "id": id }).to_string(),
    )
}
