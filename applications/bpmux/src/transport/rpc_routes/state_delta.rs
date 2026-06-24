use std::sync::Arc;

use chrono::Utc;

use crate::board::RefreshDriver;
use crate::board::domain::{Story, StoryState};
use crate::message::{Message, MessageType};
use crate::message::{StateDeltaPayload, StoryTransitionWire};
use crate::session;

pub(crate) fn fire_view_backlog_marker(session: &Arc<session::Session>) {
    let msg = Message {
        from: "bpmux:rpc".to_string(),
        text: crate::bridge::VIEW_BACKLOG_MARKER.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        crate::diag::log_diag(&format!("rpc: VIEW_BACKLOG_MARKER broadcast failed: {e}"));
    }
}

pub(crate) fn fire_board_invalidate(session: &Arc<session::Session>) {
    let delta = Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(StateDeltaPayload::BoardInvalidated {
            reason: "board changed".to_string(),
        }),
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rpc: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn fire_board_snapshot(
    session: &Arc<session::Session>,
    refresh: &Arc<dyn RefreshDriver>,
) {
    crate::transport::rest::shared::broadcast_board_snapshot(session, refresh);
}

fn effective_state_label(story: &Story) -> &'static str {
    if story.is_closed {
        if story
            .raw_labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case("retired"))
        {
            return StoryState::Retired.as_str();
        }
        return StoryState::Accepted.as_str();
    }
    story.state.as_str()
}

pub(crate) fn story_state_label(
    refresh: &Arc<dyn RefreshDriver>,
    number: u64,
) -> Option<&'static str> {
    refresh.story(number).as_ref().map(effective_state_label)
}

pub(crate) fn fire_story_transition(
    session: &Arc<session::Session>,
    number: u64,
    from_state: &str,
    target_state: &str,
    actor: &str,
) {
    let delta = Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(StateDeltaPayload::StoryTransition {
            transition: StoryTransitionWire {
                story_id: number,
                from_state: from_state.to_string(),
                target_state: target_state.to_string(),
                actor: actor.to_string(),
            },
        }),
        ..Default::default()
    };
    if let Err(e) = session.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rpc: story_state_transition state-delta broadcast failed: {e}"
        ));
    }
}
