use std::sync::Arc;

use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use message::StateDeltaPayload;
use roster::RosterAction;
use store::reactions::ReactionsAction;
use store::{AppAction, AppState};

use crate::rest::shared::state_delta_message;

pub struct RosterDeltaBroadcast {
    hub: Arc<hub::Hub>,
}

impl RosterDeltaBroadcast {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

fn member_frame(state: &AppState, routing_id: &str) -> Option<StateDeltaPayload> {
    let member = store::roster::members(&state.roster)
        .into_iter()
        .find(|m| m.routing_id == routing_id)?;
    let mut wire = roster::members_wire(std::slice::from_ref(&member), |id| {
        store::routing::role_for(&state.routing, id)
    });
    (!wire.is_empty()).then(|| StateDeltaPayload::RosterMemberUpserted {
        member: wire.remove(0),
    })
}

impl Listener<AppState, AppAction> for RosterDeltaBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let frame = match &reaction.action {
            AppAction::Roster(
                RosterAction::RosterMemberUpserted { routing_id, .. }
                | RosterAction::RosterPresenceChanged { routing_id, .. },
            ) => member_frame(&reaction.current, routing_id),
            AppAction::Roster(RosterAction::RosterMemberRemoved { routing_id }) => {
                Some(StateDeltaPayload::RosterMemberRemoved {
                    routing_id: routing_id.clone(),
                })
            }
            _ => None,
        };
        let Some(frame) = frame else {
            return Box::pin(async {});
        };
        let hub = Arc::clone(&self.hub);
        Box::pin(async move {
            hub.broadcast_ephemeral(&state_delta_message(frame));
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct ReactionDeltaBroadcast {
    hub: Arc<hub::Hub>,
}

impl ReactionDeltaBroadcast {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl Listener<AppState, AppAction> for ReactionDeltaBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let AppAction::Reactions(ReactionsAction::ReactionToggled {
            channel,
            message_key,
            emoji,
            ..
        }) = &reaction.action
        else {
            return Box::pin(async {});
        };
        let (count, users) =
            store::reactions::aggregated_for_channel(&reaction.current.reactions, channel)
                .into_iter()
                .find(|a| a.message_key == *message_key && a.emoji == *emoji)
                .map(|a| (a.count as u64, a.users))
                .unwrap_or((0, Vec::new()));
        let frame = StateDeltaPayload::ReactionUpdated {
            channel: channel.clone(),
            message_key: message_key.clone(),
            emoji: emoji.clone(),
            count,
            users,
        };
        let hub = Arc::clone(&self.hub);
        Box::pin(async move {
            hub.broadcast_ephemeral(&state_delta_message(frame));
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
