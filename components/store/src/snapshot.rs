use super::Store;

impl Store {
    pub fn build_snapshot(&self) -> message::StateSnapshotPayload {
        use message::{
            CiSnapshotWire, ReactionAggregateWire, ReactionsChannelSnapshot, StateSnapshotPayload,
        };
        let reactions = self
            .reactions_channel_keys()
            .into_iter()
            .map(|channel| {
                let aggregated = self
                    .reactions_aggregated_for_channel(&channel)
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
        let ci = self.ci_snapshot().map(|s| CiSnapshotWire {
            state: s.state.into(),
            run_id: s.run_id,
            head_sha: s.head_sha,
            display_title: s.display_title,
            last_successful_poll_at: None,
            url: Some(::ci::github_actions_url()),
        });
        let selector = self.selector();
        let roster = roster::members_wire(&self.roster_members(), |routing_id| {
            selector.select(|state| crate::routing::role_for(&state.routing, routing_id))
        });
        StateSnapshotPayload {
            reactions,
            ci,
            roster,
        }
    }
}
