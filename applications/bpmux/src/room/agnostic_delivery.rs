use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::hub::Hub;
use crate::message::{BROADCAST_ALL, Message};
use crate::room::LivePtyRegistry;
use crate::routing::short_name;
use crate::store::routing::RoutingAction;
use crate::store::{Dispatch, Select};

use super::terminal_inject::frame_team_message;

const HOLD_DEBOUNCE: Duration = Duration::from_millis(600);
const HOLD_MAX: Duration = Duration::from_secs(10);

pub struct HeldInject {
    pub session_id: String,
    pub terminal_id: String,
    pub line: String,
    pub ts: String,
}

pub fn spawn_terminal_delivery_loop(
    hub: Arc<Hub>,
    registry: Arc<LivePtyRegistry>,
    select: Select,
    dispatch: Dispatch,
) {
    let mut rx = hub.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    for inject in deliver_to_terminals(&msg, &registry, &select, &dispatch) {
                        spawn_deferred_inject(registry.clone(), dispatch.clone(), inject);
                    }
                    for inject in super::wake_delivery::wake_committed_terminals(
                        &msg, &registry, &select, &dispatch,
                    ) {
                        spawn_deferred_inject(registry.clone(), dispatch.clone(), inject);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn mark_delivered(dispatch: &Dispatch, session_id: &str, ts: &str) {
    dispatch.dispatch(
        RoutingAction::CursorAdvanced {
            session_id: session_id.to_string(),
            cursor: ts.to_string(),
        }
        .into(),
    );
}

pub(crate) fn owner_mid_typing(registry: &LivePtyRegistry, terminal_id: &str) -> bool {
    registry
        .human_input_age(terminal_id)
        .is_some_and(|age| age < HOLD_DEBOUNCE)
}

pub(crate) fn spawn_deferred_inject(
    registry: Arc<LivePtyRegistry>,
    dispatch: Dispatch,
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
                    mark_delivered(&dispatch, &inject.session_id, &inject.ts);
                }
                return;
            }
        }
    });
}

pub fn deliver_to_terminals(
    msg: &Message,
    registry: &LivePtyRegistry,
    select: &Select,
    dispatch: &Dispatch,
) -> Vec<HeldInject> {
    let mut held = Vec::new();
    if !msg.is_chat() {
        return held;
    }
    let line = framed_line(msg);
    let ts = msg.ts.to_rfc3339();
    for (session_id, terminal_id) in select.select(|s| {
        crate::store::routing::delivery_targets(&s.routing, msg.to.as_deref(), &msg.from)
    }) {
        if !after_cursor(select, &session_id, msg.ts) {
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
            mark_delivered(dispatch, &session_id, &ts);
        }
    }
    held
}

#[allow(clippy::too_many_arguments)]
pub fn drain_missed(
    hub: &Hub,
    registry: &LivePtyRegistry,
    select: &Select,
    dispatch: &Dispatch,
    session_id: &str,
    terminal_id: &str,
    display: &str,
    join_ts: &str,
) {
    let join = chrono::DateTime::parse_from_rfc3339(join_ts)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc));
    const BACKFILL_SCAN_WINDOW: usize = 1000;
    let mut messages = match hub.read_recent(BACKFILL_SCAN_WINDOW, 0, &|_| true) {
        Ok(messages) => messages,
        Err(_) => return,
    };
    messages.reverse();
    for msg in messages {
        if !msg.is_chat() || short_name(&msg.from) == display || !reaches(&msg, display) {
            continue;
        }
        if join.is_some_and(|join| msg.ts <= join) {
            continue;
        }
        if !after_cursor(select, session_id, msg.ts) {
            continue;
        }
        if registry.submit_to(terminal_id, framed_line(&msg).as_bytes()) {
            mark_delivered(dispatch, session_id, &msg.ts.to_rfc3339());
        }
    }
}

fn framed_line(msg: &Message) -> String {
    let scope = scope_label(msg.to.as_deref());
    frame_team_message(short_name(&msg.from), &scope, &msg.text)
}

fn after_cursor(select: &Select, session_id: &str, ts: chrono::DateTime<chrono::Utc>) -> bool {
    match select
        .select(|s| crate::store::routing::cursor(&s.routing, session_id))
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
