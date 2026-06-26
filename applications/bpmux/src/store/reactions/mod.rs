use std::collections::BTreeMap;

use serde::Serialize;

use crate::store::kernel::Changed;

mod listeners;

pub use listeners::ReactionsSessionBroadcaster;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AggregatedReaction {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReactionsAction {
    ReactionToggled {
        channel: String,
        message_key: String,
        emoji: String,
        user: String,
        ts_iso: String,
    },
}

type ReactorEntry = (String, String);
type EmojiBuckets = BTreeMap<(String, String), Vec<ReactorEntry>>;

#[derive(Default, Clone)]
pub struct ReactionsState {
    by_channel: BTreeMap<String, EmojiBuckets>,
}

pub fn reduce(state: &mut ReactionsState, action: &ReactionsAction) -> Changed {
    match action {
        ReactionsAction::ReactionToggled {
            channel,
            message_key,
            emoji,
            user,
            ts_iso,
        } => {
            let buckets = state.by_channel.entry(channel.clone()).or_default();
            let key = (message_key.clone(), emoji.clone());
            let reactors = buckets.entry(key.clone()).or_default();
            let existing = reactors.iter().position(|(u, _)| u == user);
            if let Some(idx) = existing {
                reactors.remove(idx);
                if reactors.is_empty() {
                    buckets.remove(&key);
                }
            } else {
                reactors.push((user.clone(), ts_iso.clone()));
            }
            if buckets.is_empty() {
                state.by_channel.remove(channel);
            }
            Changed::Yes
        }
    }
}

pub fn channel_keys(state: &ReactionsState) -> Vec<String> {
    state.by_channel.keys().cloned().collect()
}

pub fn aggregated_for_channel(state: &ReactionsState, channel: &str) -> Vec<AggregatedReaction> {
    let Some(buckets) = state.by_channel.get(channel) else {
        return Vec::new();
    };
    buckets
        .iter()
        .map(|((message_key, emoji), reactors)| {
            let users: Vec<String> = reactors.iter().map(|(u, _)| u.clone()).collect();
            AggregatedReaction {
                count: users.len(),
                message_key: message_key.clone(),
                emoji: emoji.clone(),
                users,
            }
        })
        .collect()
}

pub trait ReactionsBroadcastListener: Send + Sync {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]);
}
