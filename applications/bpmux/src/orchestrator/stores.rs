use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreReduceOutcome {
    Changed,
    Unchanged,
}

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

#[derive(Default)]
pub struct ReactionsStore {
    inner: Mutex<ReactionsState>,
}

type ReactorEntry = (String, String);
type EmojiBuckets = BTreeMap<(String, String), Vec<ReactorEntry>>;

#[derive(Default, Clone)]
struct ReactionsState {
    by_channel: BTreeMap<String, EmojiBuckets>,
}

impl ReactionsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channel_keys(&self) -> Vec<String> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.by_channel.keys().cloned().collect()
    }

    pub fn aggregated_for_channel(&self, channel: &str) -> Vec<AggregatedReaction> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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

    fn apply(&self, action: &ReactionsAction) -> StoreReduceOutcome {
        match action {
            ReactionsAction::ReactionToggled {
                channel,
                message_key,
                emoji,
                user,
                ts_iso,
            } => {
                let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
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
                StoreReduceOutcome::Changed
            }
        }
    }
}

pub trait ReactionsBroadcastListener: Send + Sync {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PinnedMessage {
    pub message_key: String,
    pub pinner: String,
    pub ts: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinsAction {
    PinAdded {
        channel: String,
        pin: PinnedMessage,
    },
    PinRemoved {
        channel: String,
        message_key: String,
    },
}

#[derive(Default)]
pub struct PinsStore {
    inner: Mutex<PinsState>,
}

#[derive(Default, Clone)]
struct PinsState {
    by_channel: BTreeMap<String, Vec<PinnedMessage>>,
}

impl PinsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn channel_keys(&self) -> Vec<String> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.by_channel.keys().cloned().collect()
    }

    pub fn list_for_channel(&self, channel: &str) -> Vec<PinnedMessage> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.by_channel.get(channel).cloned().unwrap_or_default()
    }

    fn apply(&self, action: &PinsAction) -> StoreReduceOutcome {
        match action {
            PinsAction::PinAdded { channel, pin } => {
                let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let list = state.by_channel.entry(channel.clone()).or_default();
                if list.iter().any(|p| p.message_key == pin.message_key) {
                    return StoreReduceOutcome::Unchanged;
                }
                list.push(pin.clone());
                StoreReduceOutcome::Changed
            }
            PinsAction::PinRemoved {
                channel,
                message_key,
            } => {
                let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let Some(list) = state.by_channel.get_mut(channel) else {
                    return StoreReduceOutcome::Unchanged;
                };
                let before = list.len();
                list.retain(|p| &p.message_key != message_key);
                if list.len() == before {
                    return StoreReduceOutcome::Unchanged;
                }
                if list.is_empty() {
                    state.by_channel.remove(channel);
                }
                StoreReduceOutcome::Changed
            }
        }
    }
}

pub trait PinsBroadcastListener: Send + Sync {
    fn on_changed(&self, channel: &str, pins: &[PinnedMessage]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum RosterRole {
    Designer,
    Dev,
    Pm,
    Owner,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub enum Presence {
    Online,
    Busy,
    Away,
    Offline,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RosterMember {
    pub name: String,
    pub identity: String,
    pub role: RosterRole,
    pub presence: Presence,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RosterAction {
    RosterMemberUpserted {
        name: String,
        identity: String,
        role: RosterRole,
        presence: Presence,
    },
    RosterMemberRemoved {
        name: String,
    },
    RosterPresenceChanged {
        name: String,
        presence: Presence,
    },
}

#[derive(Default)]
pub struct RosterStore {
    inner: Mutex<RosterState>,
}

#[derive(Default, Clone)]
struct RosterState {
    members: Vec<RosterMember>,
}

impl RosterStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn members(&self) -> Vec<RosterMember> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.members.clone()
    }

    fn apply(&self, action: &RosterAction) -> StoreReduceOutcome {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match action {
            RosterAction::RosterMemberUpserted {
                name,
                identity,
                role,
                presence,
            } => {
                if let Some(idx) = state.members.iter().position(|m| &m.name == name) {
                    let m = &mut state.members[idx];
                    if m.identity == *identity && m.role == *role && m.presence == *presence {
                        return StoreReduceOutcome::Unchanged;
                    }
                    m.identity = identity.clone();
                    m.role = *role;
                    m.presence = *presence;
                    StoreReduceOutcome::Changed
                } else {
                    state.members.push(RosterMember {
                        name: name.clone(),
                        identity: identity.clone(),
                        role: *role,
                        presence: *presence,
                    });
                    StoreReduceOutcome::Changed
                }
            }
            RosterAction::RosterMemberRemoved { name } => {
                let before = state.members.len();
                state.members.retain(|m| &m.name != name);
                if state.members.len() == before {
                    StoreReduceOutcome::Unchanged
                } else {
                    StoreReduceOutcome::Changed
                }
            }
            RosterAction::RosterPresenceChanged { name, presence } => {
                let Some(m) = state.members.iter_mut().find(|m| &m.name == name) else {
                    return StoreReduceOutcome::Unchanged;
                };
                if m.presence == *presence {
                    return StoreReduceOutcome::Unchanged;
                }
                m.presence = *presence;
                StoreReduceOutcome::Changed
            }
        }
    }
}

pub trait RosterBroadcastListener: Send + Sync {
    fn on_changed(&self, members: &[RosterMember]);
}

use serde::Serialize;

pub(super) fn apply_reactions(
    store: &Arc<ReactionsStore>,
    action: &ReactionsAction,
) -> StoreReduceOutcome {
    store.apply(action)
}

pub(super) fn apply_pins(store: &Arc<PinsStore>, action: &PinsAction) -> StoreReduceOutcome {
    store.apply(action)
}

pub(super) fn apply_roster(store: &Arc<RosterStore>, action: &RosterAction) -> StoreReduceOutcome {
    store.apply(action)
}
