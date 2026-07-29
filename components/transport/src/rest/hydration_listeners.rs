use std::sync::Arc;

use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use message::{CiSnapshotWire, Message, MessageType, ReactionAggregateWire, StateDeltaPayload};
use store::{AppAction, AppState, ClientConnection};

type FrameBuilder = Box<dyn Fn(&AppState, &store::Select) -> Vec<StateDeltaPayload> + Send + Sync>;

const HISTORY_CHAT_LIMIT: usize = 200;
const HISTORY_ACTIVITY_LIMIT: usize = 50;

pub struct SliceHydration {
    hub: Arc<hub::Hub>,
    select: store::Select,
    frames: FrameBuilder,
}

impl SliceHydration {
    fn listening(hub: Arc<hub::Hub>, select: store::Select, frames: FrameBuilder) -> Self {
        Self {
            hub,
            select,
            frames,
        }
    }

    pub fn roster(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self::listening(
            hub,
            select,
            Box::new(|state, _| {
                let members = roster::members_wire(&store::roster::members(&state.roster), |id| {
                    store::routing::role_for(&state.routing, id)
                });
                vec![StateDeltaPayload::Roster { members }]
            }),
        )
    }

    pub fn reactions(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self::listening(
            hub,
            select,
            Box::new(|state, _| {
                store::reactions::channel_keys(&state.reactions)
                    .into_iter()
                    .map(|channel| {
                        let aggregated =
                            store::reactions::aggregated_for_channel(&state.reactions, &channel)
                                .into_iter()
                                .map(|a| ReactionAggregateWire {
                                    message_key: a.message_key,
                                    emoji: a.emoji,
                                    count: a.count,
                                    users: a.users,
                                })
                                .collect();
                        StateDeltaPayload::Reactions {
                            channel,
                            aggregated,
                        }
                    })
                    .collect()
            }),
        )
    }

    pub fn ci(hub: Arc<hub::Hub>, select: store::Select, actions_url: ci::ActionsUrl) -> Self {
        Self::listening(
            hub,
            select,
            Box::new(move |state, _| {
                store::ci::snapshot(&state.ci)
                    .map(|s| {
                        vec![StateDeltaPayload::Ci {
                            snapshot: CiSnapshotWire {
                                state: s.state.into(),
                                run_id: s.run_id,
                                head_sha: s.head_sha,
                                display_title: s.display_title,
                                last_successful_poll_at: None,
                                url: Some(actions_url.current()),
                            },
                        }]
                    })
                    .unwrap_or_default()
            }),
        )
    }

    pub fn history(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        let ledger = Arc::clone(&hub);
        Self::listening(
            hub,
            select,
            Box::new(move |_, _| {
                let chat = ledger.read_recent(HISTORY_CHAT_LIMIT, 0, &Message::is_chat);
                let activity = ledger.read_recent(HISTORY_ACTIVITY_LIMIT, 0, &|m: &Message| {
                    m.message_type == MessageType::Activity
                });
                let (Ok(mut combined), Ok(activity)) = (chat, activity) else {
                    return Vec::new();
                };
                combined.extend(activity);
                combined.sort_by_key(|m| m.ts);
                let messages = combined
                    .iter()
                    .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                    .collect();
                vec![StateDeltaPayload::History { messages }]
            }),
        )
    }

    pub fn board(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self::listening(
            hub,
            select,
            Box::new(|_, select| {
                vec![
                    StateDeltaPayload::Stories {
                        stories: serde_json::Value::Array(super::board::board_stories(select)),
                        columns: Some(serde_json::Value::Array(super::board::board_columns_json())),
                    },
                    StateDeltaPayload::Epics {
                        epics: serde_json::Value::Array(super::board::board_epics(select)),
                    },
                    StateDeltaPayload::Outcomes {
                        outcomes: serde_json::Value::Array(super::board::board_outcomes(select)),
                    },
                    StateDeltaPayload::Iterations {
                        iterations: serde_json::Value::Array(super::board::board_iterations(
                            select,
                        )),
                    },
                    StateDeltaPayload::Roadmap {
                        roadmap: serde_json::Value::Array(super::board::board_roadmap(select)),
                    },
                ]
            }),
        )
    }
}

fn addressed(payload: StateDeltaPayload, connection: &ClientConnection) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: MessageType::StateDelta,
        state_delta: Some(payload),
        to_ids: connection.user_id.clone().map(|id| vec![id]),
        to: connection
            .user_id
            .is_none()
            .then(|| vec![connection.identity.clone()]),
        ..Message::default()
    }
}

impl Listener<AppState, AppAction> for SliceHydration {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let AppAction::ClientConnected(connection) = &reaction.action else {
            return Box::pin(async {});
        };
        let connection = connection.clone();
        let frames = (self.frames)(&reaction.current, &self.select);
        let hub = Arc::clone(&self.hub);
        Box::pin(async move {
            for payload in frames {
                hub.broadcast_ephemeral(&addressed(payload, &connection));
            }
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
