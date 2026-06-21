use crate::room::CommitmentStore;
use crate::room::LivePtyRegistry;
use crate::session::message::Message;
use crate::session::routing_identity::RoutingStore;
use crate::session::terminal_inject::frame_team_message;

use super::agnostic_delivery::{HeldInject, owner_mid_typing};

pub fn wake_committed_terminals(
    msg: &Message,
    registry: &LivePtyRegistry,
    routing: &RoutingStore,
    commitments: &CommitmentStore,
) -> Vec<HeldInject> {
    let mut held = Vec::new();
    if msg.is_chat() {
        return held;
    }
    let ts = msg.ts.to_rfc3339();
    let Some((event_class, payload)) = msg
        .state_delta
        .as_ref()
        .and_then(|delta| delta.to_subscription_event(&ts))
    else {
        return held;
    };
    let line = wake_line(event_class, &payload);
    for role in commitments.take_matching(event_class, &payload) {
        for session_id in routing.resolve(&role) {
            let Some(terminal_id) = routing.terminal_for(&session_id) else {
                continue;
            };
            if owner_mid_typing(registry, &terminal_id) {
                held.push(HeldInject {
                    session_id,
                    terminal_id,
                    line: line.clone(),
                    ts: ts.clone(),
                });
            } else {
                registry.submit_to(&terminal_id, line.as_bytes());
            }
        }
    }
    held
}

fn wake_line(event_class: &str, payload: &serde_json::Value) -> String {
    let body = format!(
        "your committed fire-on-event gate fired — act on it now: {}",
        serde_json::to_string(payload).unwrap_or_else(|_| payload.to_string())
    );
    frame_team_message("bpmux", event_class, &body)
}
