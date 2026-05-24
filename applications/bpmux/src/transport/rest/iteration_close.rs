use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{TrackerHandle, broadcast_board_invalidate, err_body, json_headers};
use crate::session;

#[derive(Default, Deserialize)]
pub struct CloseIterationRequest {
    #[serde(default)]
    pub iteration_number: Option<u64>,
}

pub async fn post_close_iteration(
    State(session): State<Arc<session::Session>>,
    axum::Extension(tracker): axum::Extension<TrackerHandle>,
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    body: Option<axum::Json<CloseIterationRequest>>,
) -> impl IntoResponse {
    let iter_n = match body.and_then(|b| b.0.iteration_number) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body("close_iteration requires iteration_number (canonical Iteration issue#)."),
            );
        }
    };
    post_close_iteration_canonical(session, tracker, test_refresh, iter_n).await
}

async fn post_close_iteration_canonical(
    session: Arc<session::Session>,
    tracker: TrackerHandle,
    test_refresh: crate::server::TestRefreshHandle,
    iteration_number: u64,
) -> (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
) {
    let Some(refresh) = test_refresh.as_ref() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            json_headers(),
            err_body(
                "close_iteration(canonical) requires refresh-driver — server has no canonical store available",
            ),
        );
    };
    let iteration = match refresh.story(iteration_number) {
        Some(s) if matches!(s.story_type, crate::board::domain::StoryType::Iteration) => s,
        Some(s) => {
            return (
                StatusCode::BAD_REQUEST,
                json_headers(),
                err_body(&format!(
                    "close_iteration: #{iteration_number} is not an Iteration (story_type={:?})",
                    s.story_type
                )),
            );
        }
        None => {
            return (
                StatusCode::NOT_FOUND,
                json_headers(),
                err_body(&format!(
                    "close_iteration: iteration_number #{iteration_number} not found"
                )),
            );
        }
    };
    if iteration.is_closed {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body(&format!(
                "close_iteration: iteration #{iteration_number} is already closed"
            )),
        );
    }
    let theme_name = crate::board::iteration_label::theme_name_from_title(&iteration.title);
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let promoted: Vec<u64> = crate::board::iteration_label::members_with_iteration_label(
        &codec,
        &refresh.all_stories(),
        &theme_name,
    );
    let ready_queue_blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
    if !ready_queue_blockers.is_empty() {
        let body = build_close_iteration_ready_queue_blocked_body(&ready_queue_blockers);
        broadcast_iteration_close_blocked(&session, &ready_queue_blockers);
        return (StatusCode::CONFLICT, json_headers(), body);
    }
    match promote_members_then_close_iteration(refresh, iteration_number, &promoted).await {
        Ok(()) => {}
        Err(rollback_summary) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                json_headers(),
                err_body(&format!(
                    "close_iteration(canonical) failed: {rollback_summary}"
                )),
            );
        }
    }
    let payload = session::message::StateDeltaPayload::IterationClose {
        promoted: promoted.clone(),
        deleted_marker_id: None,
    };
    let delta_msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Some(t) = tracker.as_ref() {
        for n in promoted.iter() {
            let body = "iteration-close: promoted to ready via canonical-via-GH ceremony (#1300; iteration_id substrate per slice 2).";
            if let Err(e) = t.post_story_comment(*n, body).await {
                session::diag::log_diag(&format!(
                    "[#1300] comment_on_story #{n} (promoted) failed: {e}"
                ));
            }
        }
    }
    if let Err(e) = session.broadcast(&delta_msg) {
        session::diag::log_diag(&format!(
            "post_close_iteration_canonical: state-delta broadcast failed: {e}"
        ));
    }
    broadcast_board_invalidate(&session);
    let body = serde_json::json!({
        "promoted": promoted,
        "deleted_marker_id": serde_json::Value::Null,
        "iteration_number": iteration_number,
    });
    (StatusCode::OK, json_headers(), body.to_string())
}

pub(crate) async fn collect_ready_queue_blockers(
    refresh: &dyn crate::board::RefreshDriver,
) -> Vec<IterationCloseBlocker> {
    refresh
        .all_stories()
        .into_iter()
        .filter(|s| matches!(s.state, crate::board::domain::StoryState::Ready) && !s.is_closed)
        .map(|s| IterationCloseBlocker {
            number: s.number,
            title: s.title,
            state: "ready".to_string(),
        })
        .collect()
}

fn build_close_iteration_ready_queue_blocked_body(blockers: &[IterationCloseBlocker]) -> String {
    serde_json::json!({
        "error": "close_iteration blocked: ready queue must be empty before canonical-via-GH iteration close (#1258 r3 guard).",
        "blockers": blockers,
    })
    .to_string()
}

pub(crate) async fn promote_members_then_close_iteration(
    refresh: &Arc<dyn crate::board::RefreshDriver>,
    iteration_number: u64,
    promoted: &[u64],
) -> Result<(), String> {
    let iteration_story = refresh
        .story(iteration_number)
        .ok_or_else(|| format!("iteration #{iteration_number} not found in store"))?;
    let theme_name = crate::board::iteration_label::theme_name_from_title(&iteration_story.title);
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let mut applied: Vec<(u64, String)> = Vec::with_capacity(promoted.len());
    for &n in promoted {
        let member = refresh
            .story(n)
            .ok_or_else(|| format!("member #{n} not found in store"))?;
        let member_label = crate::board::iteration_label::story_iteration_label_for_theme(
            &codec,
            &member,
            &theme_name,
        )
        .ok_or_else(|| {
            format!("member #{n} carries no iteration-{theme_name}.* label (substrate-disconnect)")
        })?;
        let result: Result<(), String> = (|| {
            refresh
                .mutate_sync(crate::board::BoardEvent::TagAdded {
                    meta: meta(),
                    number: n,
                    label: "ready".to_string(),
                })
                .map_err(|e| format!("#{n} ready-tag enqueue failed: {e}"))?;
            refresh
                .mutate_sync(crate::board::BoardEvent::StateTransitioned {
                    meta: meta(),
                    number: n,
                    new_state: crate::board::domain::StoryState::Ready,
                })
                .map_err(|e| format!("#{n} state-transition enqueue failed: {e}"))?;
            refresh
                .mutate_sync(crate::board::BoardEvent::TagRemoved {
                    meta: meta(),
                    number: n,
                    label: member_label.clone(),
                })
                .map_err(|e| format!("#{n} iteration-label-remove enqueue failed: {e}"))?;
            Ok(())
        })();
        match result {
            Ok(()) => applied.push((n, member_label)),
            Err(e) => {
                rollback_promoted_members(refresh, &applied);
                return Err(format!("promote failed at #{n}: {e} (rolled back)"));
            }
        }
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::CurrentIterationCleared {
            meta: meta(),
            number: iteration_number,
        })
        .map_err(|e| {
            rollback_promoted_members(refresh, &applied);
            format!(
                "#{iteration_number} current-designation-clear enqueue failed: {e} (members rolled back)"
            )
        })?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta: meta(),
            number: iteration_number,
            reason: Some(format!(
                "iteration-close (#1300 canonical-via-GH; {} members promoted)",
                applied.len()
            )),
        })
        .map_err(|e| {
            rollback_promoted_members(refresh, &applied);
            format!("#{iteration_number} iteration-close enqueue failed: {e} (members rolled back; iteration-issue unchanged)")
        })?;
    Ok(())
}

fn rollback_promoted_members(
    refresh: &Arc<dyn crate::board::RefreshDriver>,
    applied: &[(u64, String)],
) {
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    for (n, label) in applied {
        let _ = refresh.mutate_sync(crate::board::BoardEvent::TagRemoved {
            meta: meta(),
            number: *n,
            label: "ready".to_string(),
        });
        let _ = refresh.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta: meta(),
            number: *n,
            new_state: crate::board::domain::StoryState::Icebox,
        });
        let _ = refresh.mutate_sync(crate::board::BoardEvent::TagAdded {
            meta: meta(),
            number: *n,
            label: label.clone(),
        });
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct IterationCloseBlocker {
    pub number: u64,
    pub title: String,
    pub state: String,
}

fn broadcast_iteration_close_blocked(
    session: &Arc<session::Session>,
    blockers: &[IterationCloseBlocker],
) {
    let payload = session::message::StateDeltaPayload::IterationCloseBlocked {
        blockers: blockers
            .iter()
            .map(|b| session::message::IterationCloseBlockerPayload {
                number: b.number,
                title: b.title.clone(),
                state: b.state.clone(),
            })
            .collect(),
    };
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!(
            "post_close_iteration: iteration-close-blocked broadcast failed: {e}"
        ));
    }
}
