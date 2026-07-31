use std::sync::Arc;
use std::time::Duration;

use tokio::sync::broadcast;

use crate::LivePtyRegistry;
use hub::Hub;
use message::Message;
use store::routing::RoutingAction;
use store::{Dispatch, Select};

const HOLD_DEBOUNCE: Duration = Duration::from_millis(600);
const HOLD_MAX: Duration = Duration::from_secs(10);

pub struct HeldInject {
    pub user_id: String,
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
                Ok(msg) => deliver_message(&msg, &registry, &select, &dispatch),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn deliver_message(
    msg: &Message,
    registry: &Arc<LivePtyRegistry>,
    select: &Select,
    dispatch: &Dispatch,
) {
    for inject in deliver_to_terminals(msg, registry, select, dispatch) {
        spawn_deferred_inject(registry.clone(), dispatch.clone(), inject);
    }
    for inject in super::wake_delivery::wake_committed_terminals(msg, registry, select, dispatch) {
        spawn_deferred_inject(registry.clone(), dispatch.clone(), inject);
    }
}

fn mark_delivered(dispatch: &Dispatch, user_id: &str, ts: &str) {
    dispatch.dispatch(
        RoutingAction::CursorAdvanced {
            user_id: user_id.to_string(),
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
            if !owner_paused && start.elapsed() < HOLD_MAX {
                continue;
            }
            if registry.submit_to(&inject.terminal_id, inject.line.as_bytes()) {
                mark_delivered(&dispatch, &inject.user_id, &inject.ts);
            }
            return;
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
    let line = super::terminal_inject::framed_line(msg);
    let ts = msg.ts.to_rfc3339();
    for (user_id, terminal_id) in select.select(|state| {
        store::routing::delivery_targets(
            &state.routing,
            msg.to.as_deref(),
            msg.to_ids.as_deref(),
            &msg.from,
            msg.sender_id.as_deref(),
            msg.sender_terminal.as_deref(),
        )
    }) {
        if !after_cursor(select, &user_id, msg.ts) {
            continue;
        }
        if owner_mid_typing(registry, &terminal_id) {
            held.push(HeldInject {
                user_id,
                terminal_id,
                line: line.clone(),
                ts: ts.clone(),
            });
        } else if registry.submit_to(&terminal_id, line.as_bytes()) {
            mark_delivered(dispatch, &user_id, &ts);
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
    user_id: &str,
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
        if !msg.is_chat()
            || msg.is_from_user(Some(user_id), display)
            || !msg.reaches_user(Some(user_id), display)
            || msg.sender_terminal.as_deref() == Some(terminal_id)
        {
            continue;
        }
        if join.is_some_and(|join| msg.ts <= join) {
            continue;
        }
        if !after_cursor(select, user_id, msg.ts) {
            continue;
        }
        if registry.submit_to(
            terminal_id,
            super::terminal_inject::framed_line(&msg).as_bytes(),
        ) {
            mark_delivered(dispatch, user_id, &msg.ts.to_rfc3339());
        }
    }
}

fn after_cursor(select: &Select, user_id: &str, ts: chrono::DateTime<chrono::Utc>) -> bool {
    match select
        .select(|state| store::routing::cursor(&state.routing, user_id))
        .as_deref()
        .and_then(|cursor| chrono::DateTime::parse_from_rfc3339(cursor).ok())
    {
        Some(cursor) => ts > cursor.with_timezone(&chrono::Utc),
        None => true,
    }
}
