use super::terminal_inject::frame_team_message;
use crate::room::LivePtyRegistry;
use crate::store::commitments::matching_users;
use crate::store::{CommitmentsAction, Dispatch, Select};
use message::Message;

use super::agnostic_delivery::{HeldInject, owner_mid_typing};

pub fn wake_committed_terminals(
    msg: &Message,
    registry: &LivePtyRegistry,
    select: &Select,
    dispatch: &Dispatch,
) -> Vec<HeldInject> {
    let mut held = Vec::new();
    if msg.is_chat() {
        return held;
    }
    let ts = msg.ts.to_rfc3339();
    let Some((event_class, payload)) = msg
        .state_delta
        .as_ref()
        .and_then(|delta| delta.to_subscription_event(&ts, &crate::ci::github_actions_url()))
    else {
        return held;
    };
    let line = wake_line(event_class, &payload);
    let woken = select.select(|state| matching_users(&state.commitments, event_class, &payload));
    dispatch.dispatch(
        CommitmentsAction::CommitmentsMatched {
            user_ids: woken.clone(),
            event_class: event_class.to_string(),
            payload: payload.clone(),
        }
        .into(),
    );
    for user_id in woken {
        let Some(terminal_id) =
            select.select(|state| crate::store::routing::terminal_for(&state.routing, &user_id))
        else {
            continue;
        };
        if owner_mid_typing(registry, &terminal_id) {
            held.push(HeldInject {
                user_id,
                terminal_id,
                line: line.clone(),
                ts: ts.clone(),
            });
        } else {
            registry.submit_to(&terminal_id, line.as_bytes());
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
