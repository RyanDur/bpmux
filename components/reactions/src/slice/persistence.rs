use std::collections::BTreeMap;

use crate::{Reaction, ReactionsStore};

use super::{EmojiBuckets, ReactionsState};

pub fn to_store(state: &ReactionsState) -> ReactionsStore {
    let mut channels: BTreeMap<String, Vec<Reaction>> = BTreeMap::new();
    for (channel, buckets) in &state.by_channel {
        let mut list = Vec::new();
        for ((message_key, emoji), reactors) in buckets {
            for (user, reacted_at) in reactors {
                list.push(Reaction {
                    message_key: message_key.clone(),
                    emoji: emoji.clone(),
                    user: user.clone(),
                    reacted_at: reacted_at.clone(),
                });
            }
        }
        channels.insert(channel.clone(), list);
    }
    ReactionsStore { channels }
}

pub fn from_store(store: &ReactionsStore) -> ReactionsState {
    let mut by_channel: BTreeMap<String, EmojiBuckets> = BTreeMap::new();
    for (channel, list) in &store.channels {
        let buckets = by_channel.entry(channel.clone()).or_default();
        for reaction in list {
            buckets
                .entry((reaction.message_key.clone(), reaction.emoji.clone()))
                .or_default()
                .push((reaction.user.clone(), reaction.reacted_at.clone()));
        }
    }
    ReactionsState { by_channel }
}
