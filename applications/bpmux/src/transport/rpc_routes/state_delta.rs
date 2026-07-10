use std::sync::Arc;

use chrono::Utc;

use crate::hub;
use crate::sync::RefreshDriver;
use message::{Message, MessageType};
use message::{StateDeltaPayload, StoryTransitionWire};

pub(crate) fn fire_view_backlog_marker(hub: &Arc<hub::Hub>) {
    let msg = Message {
        from: "bpmux:rpc".to_string(),
        text: crate::bridge::VIEW_BACKLOG_MARKER.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Default::default()
    };
    if let Err(e) = hub.broadcast(&msg) {
        crate::diag::log_diag(&format!("rpc: VIEW_BACKLOG_MARKER broadcast failed: {e}"));
    }
}

pub(crate) fn fire_board_invalidate(hub: &Arc<hub::Hub>) {
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
    if let Err(e) = hub.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rpc: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub(crate) fn fire_stories(hub: &Arc<hub::Hub>, refresh: &Arc<dyn RefreshDriver>) {
    crate::transport::rest::shared::broadcast_stories(hub, refresh);
}

pub(crate) fn story_state_label(
    refresh: &Arc<dyn RefreshDriver>,
    number: u64,
) -> Option<&'static str> {
    refresh.story(number).map(|story| {
        crate::product_management::scheduling::projected_state(&story, &refresh.iterations())
            .as_str()
    })
}

pub(crate) fn fire_story_transition(
    hub: &Arc<hub::Hub>,
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
    if let Err(e) = hub.broadcast(&delta) {
        crate::diag::log_diag(&format!(
            "rpc: story_state_transition state-delta broadcast failed: {e}"
        ));
    }
}
