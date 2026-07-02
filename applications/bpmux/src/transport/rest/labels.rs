use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{broadcast_board_invalidate, broadcast_stories, err_body, json_headers};
use crate::product_management::board::label_codec::LabelCodec;
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
                let event = if let Some(tag) = codec.decode_tag(&label) {
                    Some(crate::product_management::board::StoryEvent::TagSet {
                        meta,
                        id: story_id,
                        tag,
                    })
                } else if let Some(blocker) = codec.decode_blocked_by(&label) {
                    Some(crate::product_management::board::StoryEvent::BlockedBySet {
                        meta,
                        id: story_id,
                        blocker: crate::product_management::story::StoryId::new(blocker),
                    })
                } else if let Some(parts) =
                    crate::product_management::iteration::label::parse_iteration_label(
                        &codec, &label,
                    )
                {
                    Some(
                        crate::product_management::board::StoryEvent::IterationSlotSet {
                            meta,
                            id: story_id,
                            name: parts.name,
                            position: parts.position,
                        },
                    )
                } else {
                    Some(crate::product_management::board::StoryEvent::TagAdded {
                        meta,
                        id: story_id,
                        label,
                    })
                };
                (event, descriptor)
            }
            LabelOp::Remove(label) => {
                let descriptor = format!("TagRemoved {label} on #{id}");
                let event = if let Some(tag) = codec.decode_tag(&label) {
                    Some(crate::product_management::board::StoryEvent::TagUnset {
                        meta,
                        id: story_id,
                        tag,
                    })
                } else if codec.decode_blocked_by(&label).is_some() {
                    Some(
                        crate::product_management::board::StoryEvent::BlockedByCleared {
                            meta,
                            id: story_id,
                        },
                    )
                } else if codec.is_iteration_label_loose(&label) {
                    let current_chip = refresh.story(id).and_then(|s| {
                        s.iteration_slot()
                            .map(|slot| codec.encode_iteration(&slot.name, slot.position))
                    });
                    if current_chip.as_deref() == Some(label.as_str()) {
                        Some(
                            crate::product_management::board::StoryEvent::IterationSlotCleared {
                                meta,
                                id: story_id,
                            },
                        )
                    } else {
                        None
                    }
                } else {
                    Some(crate::product_management::board::StoryEvent::TagRemoved {
                        meta,
                        id: story_id,
                        label,
                    })
                };
                (event, descriptor)
            }
        };
        let Some(event) = event else {
            continue;
        };
        if let Err(e) = refresh.mutate_sync(event) {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!("{descriptor}: {e}")),
            );
        }
    }
    if let Some(mut iteration) =
        refresh.iteration(crate::product_management::iteration::IterationId::new(id))
    {
        let codec = crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
        for label in &payload.add {
            if let Some(order) = codec.decode_iteration_order(label) {
                iteration.iteration_order = Some(order);
            } else if let Some(role) = codec.decode_iteration_review_closer(label) {
                if !iteration.review_closers.contains(&role) {
                    iteration.review_closers.push(role);
                }
            } else if let Some(role) = codec.decode_iteration_dev_ok(label) {
                if !iteration.dev_ok_by.contains(&role) {
                    iteration.dev_ok_by.push(role);
                }
            } else if codec.is_current_iteration_label(label) {
                iteration.is_current = true;
            }
        }
        for label in &payload.remove {
            if let Some(order) = codec.decode_iteration_order(label) {
                if iteration.iteration_order == Some(order) {
                    iteration.iteration_order = None;
                }
            } else if let Some(role) = codec.decode_iteration_review_closer(label) {
                iteration.review_closers.retain(|r| r != &role);
            } else if let Some(role) = codec.decode_iteration_dev_ok(label) {
                iteration.dev_ok_by.retain(|r| r != &role);
            } else if codec.is_current_iteration_label(label) {
                iteration.is_current = false;
            }
        }
        dispatcher.dispatch(crate::store::AppAction::Iteration(
            crate::product_management::iteration::IterationAction::Upserted(vec![iteration]),
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
