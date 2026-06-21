use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::room::CommitmentStore;
use crate::room::LivePtyRegistry;
use crate::session::Session;
use crate::session::message::{BROADCAST_ALL, Message};
use crate::session::routing_identity::RoutingStore;
use crate::session::terminal_inject::frame_team_message;
use crate::transport::ws::short_name;

const HOLD_DEBOUNCE: Duration = Duration::from_millis(600);
const HOLD_MAX: Duration = Duration::from_secs(10);

pub struct HeldInject {
    pub session_id: String,
    pub terminal_id: String,
    pub line: String,
    pub ts: String,
}

pub fn spawn_terminal_delivery_loop(
    session: Arc<Session>,
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
    commitments: Arc<CommitmentStore>,
) {
    let mut rx = session.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    for inject in deliver_to_terminals(&msg, &registry, &routing) {
                        spawn_deferred_inject(registry.clone(), routing.clone(), inject);
                    }
                    for inject in wake_committed_terminals(&msg, &registry, &routing, &commitments)
                    {
                        spawn_deferred_inject(registry.clone(), routing.clone(), inject);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn owner_mid_typing(registry: &LivePtyRegistry, terminal_id: &str) -> bool {
    registry
        .human_input_age(terminal_id)
        .is_some_and(|age| age < HOLD_DEBOUNCE)
}

pub(crate) fn spawn_deferred_inject(
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
    inject: HeldInject,
) {
    tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        loop {
            tokio::time::sleep(HOLD_DEBOUNCE).await;
            let owner_paused = registry
                .human_input_age(&inject.terminal_id)
                .is_none_or(|age| age >= HOLD_DEBOUNCE);
            if owner_paused || start.elapsed() >= HOLD_MAX {
                if registry.submit_to(&inject.terminal_id, inject.line.as_bytes()) {
                    routing.mark_delivered(&inject.session_id, &inject.ts);
                }
                return;
            }
        }
    });
}

pub fn deliver_to_terminals(
    msg: &Message,
    registry: &LivePtyRegistry,
    routing: &RoutingStore,
) -> Vec<HeldInject> {
    let mut held = Vec::new();
    if !msg.is_chat() {
        return held;
    }
    let line = framed_line(msg);
    let ts = msg.ts.to_rfc3339();
    for (session_id, terminal_id) in routing.delivery_targets(msg.to.as_deref(), &msg.from) {
        if !after_cursor(routing, &session_id, msg.ts) {
            continue;
        }
        if owner_mid_typing(registry, &terminal_id) {
            held.push(HeldInject {
                session_id,
                terminal_id,
                line: line.clone(),
                ts: ts.clone(),
            });
        } else if registry.submit_to(&terminal_id, line.as_bytes()) {
            routing.mark_delivered(&session_id, &ts);
        }
    }
    held
}

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

pub fn drain_missed(
    session: &Session,
    registry: &LivePtyRegistry,
    routing: &RoutingStore,
    session_id: &str,
    terminal_id: &str,
    display: &str,
    join_ts: &str,
) {
    let join = chrono::DateTime::parse_from_rfc3339(join_ts)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));
    let messages = match session.read_messages() {
        Ok(messages) => messages,
        Err(_) => return,
    };
    for msg in messages {
        if !msg.is_chat() || short_name(&msg.from) == display || !reaches(&msg, display) {
            continue;
        }
        if join.is_some_and(|join| msg.ts <= join) {
            continue;
        }
        if !after_cursor(routing, session_id, msg.ts) {
            continue;
        }
        if registry.submit_to(terminal_id, framed_line(&msg).as_bytes()) {
            routing.mark_delivered(session_id, &msg.ts.to_rfc3339());
        }
    }
}

fn framed_line(msg: &Message) -> String {
    let scope = scope_label(msg.to.as_deref());
    frame_team_message(short_name(&msg.from), &scope, &msg.text)
}

fn after_cursor(
    routing: &RoutingStore,
    session_id: &str,
    ts: chrono::DateTime<chrono::Utc>,
) -> bool {
    match routing
        .cursor(session_id)
        .as_deref()
        .and_then(|cursor| chrono::DateTime::parse_from_rfc3339(cursor).ok())
    {
        Some(cursor) => ts > cursor.with_timezone(&chrono::Utc),
        None => true,
    }
}

fn reaches(msg: &Message, display: &str) -> bool {
    match msg.to.as_deref() {
        None => true,
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => true,
        Some(names) => names.iter().any(|name| short_name(name) == display),
    }
}

fn scope_label(to: Option<&[String]>) -> String {
    match to {
        None => "broadcast".to_string(),
        Some(names) if names.iter().any(|name| name == BROADCAST_ALL) => "broadcast".to_string(),
        Some(names) if names.len() == 1 => "→you".to_string(),
        Some(names) => format!(
            "to:{}",
            names
                .iter()
                .map(|name| short_name(name))
                .collect::<Vec<_>>()
                .join(",")
        ),
    }
}
