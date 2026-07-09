use super::Store;

impl Store {
    pub fn build_snapshot(&self) -> crate::message::StateSnapshotPayload {
        use crate::message::{
            CiSnapshotWire, ReactionAggregateWire, ReactionsChannelSnapshot, RosterMemberWire,
            StateSnapshotPayload,
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
            url: Some(crate::ci::github_actions_url()),
        });
        let roster = self
            .roster_members()
            .into_iter()
            .map(|m| RosterMemberWire {
                routing_id: m.routing_id,
                name: crate::roster::display_name(&m.identity).to_string(),
                role: crate::roster::role_of(&m.identity).into(),
                identity: m.identity,
                presence: m.presence.into(),
            })
            .collect();
        StateSnapshotPayload {
            reactions,
            ci,
            roster,
        }
    }
}
