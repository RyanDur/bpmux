use super::{CiPollFetch, Orchestrator};
use crate::session;

impl<T: CiPollFetch> Orchestrator<T> {
    pub fn build_snapshot(&self) -> crate::session::message::StateSnapshotPayload {
        use crate::session::message::{
            CiSnapshotWire, ReactionAggregateWire, ReactionsChannelSnapshot, RosterMemberWire,
            StateSnapshotPayload,
        };
        let reactions = self
            .reactions_store
            .channel_keys()
            .into_iter()
            .map(|channel| {
                let aggregated = self
                    .reactions_store
                    .aggregated_for_channel(&channel)
                    .into_iter()
                    .map(|a| ReactionAggregateWire {
                        message_key: a.message_key,
                        emoji: a.emoji,
                        count: a.count,
                        users: a.users,
                    })
                    .collect();
                ReactionsChannelSnapshot {
                    channel,
                    aggregated,
                }
            })
            .collect();
        let ci = self.ci_store.snapshot().map(|s| CiSnapshotWire {
            state: s.state,
            run_id: s.run_id,
            head_sha: s.head_sha,
            display_title: s.display_title,
            last_successful_poll_at: None,
        });
        let roster = self
            .roster_store
            .members()
            .into_iter()
            .map(|m| RosterMemberWire {
                routing_id: m.routing_id,
                name: m.name,
                identity: m.identity,
                role: m.role,
                presence: m.presence,
            })
            .collect();
        StateSnapshotPayload {
            reactions,
            ci,
            roster,
        }
    }

    pub fn broadcast_ci_poll(&self, last_successful_poll_at: Option<String>) {
        let snap = self.ci_store.snapshot();
        let wire = crate::session::message::CiSnapshotWire {
            state: snap
                .as_ref()
                .map(|s| s.state)
                .unwrap_or(crate::ci::CiState::Unknown),
            run_id: snap.as_ref().and_then(|s| s.run_id),
            head_sha: snap.as_ref().and_then(|s| s.head_sha.clone()),
            display_title: snap.as_ref().and_then(|s| s.display_title.clone()),
            last_successful_poll_at,
        };
        let msg = session::Message {
            from: "bpmux".to_string(),
            text: String::new(),
            ts: chrono::Utc::now(),
            message_type: session::MessageType::StateDelta,
            state_delta: Some(crate::session::message::StateDeltaPayload::CiPoll {
                snapshot: wire,
            }),
            ..session::Message::default()
        };
        self.session.broadcast_ephemeral(&msg);
    }
}
