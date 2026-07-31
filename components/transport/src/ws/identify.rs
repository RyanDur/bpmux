use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures::stream::{SplitSink, SplitStream};

use user_routing::short_name;

use super::roster::{dispatch_roster_join, dispatch_roster_leave};
use super::session_io::wait_for_identify;

pub(super) fn resolve_routing_user_id(
    store: &Arc<store::Store>,
    dispatch: &store::Dispatch,
    identity: &str,
    identify_user_id: Option<String>,
) -> Option<String> {
    match identify_user_id {
        Some(user_id) => {
            let known = store
                .selector()
                .select(|state| store::routing::display_for(&state.routing, &user_id).is_some());
            if !known {
                dispatch.dispatch(
                    store::routing::RoutingAction::Registered {
                        user_id: user_id.clone(),
                        display: short_name(identity).to_string(),
                        role: None,
                    }
                    .into(),
                );
            }
            Some(user_id)
        }
        None => store
            .selector()
            .select(|state| store::routing::resolve(&state.routing, short_name(identity)))
            .into_iter()
            .next(),
    }
}

pub(super) fn roster_key_for(routing_user_id: &Option<String>, is_owner: bool) -> Option<String> {
    if is_owner {
        return None;
    }
    routing_user_id.clone()
}

pub(super) type OpenedSession = (String, Option<String>, bool, Option<String>, Option<String>);

pub(super) async fn open_session(
    tx: &mut SplitSink<WebSocket, WsMessage>,
    rx: &mut SplitStream<WebSocket>,
    hub: &hub::Hub,
    store: &Arc<store::Store>,
    dispatch: &store::Dispatch,
) -> Result<OpenedSession, Box<dyn std::error::Error + Send + Sync>> {
    let (identity, replay_cursor, identify_user_id, identify_owner_nonce) =
        wait_for_identify(tx, rx).await?;
    let routing_user_id = resolve_routing_user_id(store, dispatch, &identity, identify_user_id);
    let claimed_owner_nonce = store
        .selector()
        .select(|state| owner_session::claimed_nonce(&state.owner_session));
    let is_owner = owner_session::resolve_owner_mark(
        claimed_owner_nonce.as_deref(),
        identify_owner_nonce.as_deref(),
        &identity,
    );
    let roster_key = roster_key_for(&routing_user_id, is_owner);
    hub.peer_connected();
    if let Some(ref key) = roster_key {
        dispatch_roster_join(store, key, &identity).await;
    }
    Ok((
        identity,
        routing_user_id,
        is_owner,
        roster_key,
        replay_cursor,
    ))
}

pub(super) async fn rebind_roster(
    store: &Arc<store::Store>,
    dispatch: &store::Dispatch,
    routing_user_id: &Option<String>,
    is_owner: bool,
    identity: &str,
    old_key: Option<String>,
) -> Option<String> {
    if let Some(user_id) = routing_user_id {
        dispatch.dispatch(
            store::routing::RoutingAction::Renamed {
                user_id: user_id.clone(),
                display: short_name(identity).to_string(),
            }
            .into(),
        );
    }
    let new_key = roster_key_for(routing_user_id, is_owner);
    if new_key != old_key
        && let Some(ref old) = old_key
    {
        dispatch_roster_leave(store, old).await;
    }
    if let Some(ref key) = new_key {
        dispatch_roster_join(store, key, identity).await;
    }
    new_key
}
