use std::sync::Arc;

use crate::session;

#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct IterationCloseBlocker {
    pub number: u64,
    pub title: String,
    pub state: String,
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

pub(super) fn build_close_iteration_ready_queue_blocked_body(
    blockers: &[IterationCloseBlocker],
) -> String {
    serde_json::json!({
        "error": "close_iteration blocked: ready queue must be empty before canonical-via-GH iteration close (#1258 r3 guard).",
        "blockers": blockers,
    })
    .to_string()
}

pub(super) fn broadcast_iteration_close_blocked(
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
        crate::diag::log_diag(&format!(
            "post_close_iteration: iteration-close-blocked broadcast failed: {e}"
        ));
    }
}
