use std::sync::Arc;

use chrono::Utc;

use message::{Message, MessageType};
use message::{StateDeltaPayload, StoryTransitionWire};

pub fn fire_view_backlog_marker(hub: &Arc<hub::Hub>) {
    let msg = Message {
        from: "bpmux:rpc".to_string(),
        text: bridge::VIEW_BACKLOG_MARKER.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Default::default()
    };
    if let Err(e) = hub.broadcast(&msg) {
        diag::log_diag(&format!("rpc: VIEW_BACKLOG_MARKER broadcast failed: {e}"));
    }
}

pub fn fire_board_invalidate(hub: &Arc<hub::Hub>) {
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
        diag::log_diag(&format!(
            "rpc: board_invalidated state-delta broadcast failed: {e}"
        ));
    }
}

pub fn story_state_label(select: &store::Select, number: u64) -> Option<&'static str> {
    let (story, iterations) = select.select(|state| {
        (
            product_management::story::slice::get(
                &state.stories,
                product_management::story::StoryId::new(number),
            ),
            product_management::iteration::all(&state.iterations),
        )
    });
    story.map(|story| product_management::scheduling::projected_state(&story, &iterations).as_str())
}

pub fn fire_story_transition(
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
        diag::log_diag(&format!(
            "rpc: story_state_transition state-delta broadcast failed: {e}"
        ));
    }
}
