use std::sync::Arc;

use tokio::sync::broadcast;

use crate::room::LivePtyRegistry;
use crate::session::Session;
use crate::session::message::{BROADCAST_ALL, Message};
use crate::session::routing_identity::RoutingStore;
use crate::session::terminal_inject::frame_team_message;
use crate::transport::ws::short_name;

pub fn spawn_terminal_delivery_loop(
    session: Arc<Session>,
    registry: Arc<LivePtyRegistry>,
    routing: Arc<RoutingStore>,
) {
    let mut rx = session.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => deliver_to_terminals(&msg, &registry, &routing),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

pub fn deliver_to_terminals(msg: &Message, registry: &LivePtyRegistry, routing: &RoutingStore) {
    if !msg.is_chat() {
        return;
    }
    let line = framed_line(msg);
    let ts = msg.ts.to_rfc3339();
    for (session_id, terminal_id) in routing.delivery_targets(msg.to.as_deref(), &msg.from) {
        if !after_cursor(routing, &session_id, msg.ts) {
            continue;
        }
        if registry.submit_to(&terminal_id, line.as_bytes()) {
            routing.mark_delivered(&session_id, &ts);
        }
    }
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
