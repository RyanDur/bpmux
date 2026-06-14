use std::sync::Arc;

use crate::board::domain::{Acceptor, StoryState};
use crate::board::iteration_label::current_iteration_number;
use crate::board::{BoardEvent, BoardStore, Story};
use crate::ci::RunEntry;
use crate::orchestrator::ci::{CiPollFetch, Orchestrator, OrchestratorAction};
use crate::orchestrator::stores::{Presence, RosterAction};
use crate::orchestrator::{
    AggregatedReaction, CiBroadcastListener, CiSnapshot, ReactionsBroadcastListener,
    RosterBroadcastListener, RosterMember,
};
use crate::session;
use crate::session::message::{
    CiSnapshotWire, ReactionAggregateWire, RosterMemberWire, StateDeltaPayload,
};
use crate::transport::ci_state_store::CiStateStore;

pub struct CiStateStoreWriter {
    store: CiStateStore,
}

impl CiStateStoreWriter {
    pub fn new(store: CiStateStore) -> Self {
        Self { store }
    }
}

impl CiBroadcastListener for CiStateStoreWriter {
    fn on_changed(&self, snap: &CiSnapshot) {
        let runs = vec![RunEntry {
            status: snap.conclusion.to_string(),
            conclusion: Some(snap.conclusion.to_string()),
            head_sha: snap.head_sha.clone(),
            display_title: snap.display_title.clone(),
            database_id: snap.run_id,
            name: None,
            annotation_message: None,
        }];
        let report = crate::bridge::tools::build_ci_state_report(&runs);
        self.store.write(report);
    }
}

pub struct CiSessionBroadcaster {
    session: Arc<session::Session>,
}

impl CiSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl CiBroadcastListener for CiSessionBroadcaster {
    fn on_changed(&self, snap: &CiSnapshot) {
        let event = snap.to_ci_event();
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Ci {
                snapshot: ci_to_wire(snap),
            },
            "ci_listener",
        );
        let Some(text) = crate::ci::compute_broadcast_text(&event) else {
            return;
        };
        let multi_depth = crate::ci::build_multi_depth_for_transition(&event);
        let activity_msg = session::Message {
            from: "bpmux".to_string(),
            text: text.clone(),
            ts: chrono::Utc::now(),
            message_type: session::MessageType::Activity,
            multi_depth: Some(multi_depth),
            ..session::Message::default()
        };
        if let Err(e) = self.session.broadcast(&activity_msg) {
            session::diag::log_diag(&format!("ci_listener: activity broadcast failed: {e}"));
        }
        let dev_signal = crate::ci::build_dev_ci_signal(&text);
        if let Err(e) = self.session.broadcast(&dev_signal) {
            session::diag::log_diag(&format!("ci_listener: dev signal failed: {e}"));
        }
    }
}

fn broadcast_state_delta(session: &session::Session, payload: StateDeltaPayload, label: &str) {
    let msg = session::Message {
        from: "bpmux".to_string(),
        text: String::new(),
        ts: chrono::Utc::now(),
        message_type: session::MessageType::StateDelta,
        state_delta: Some(payload),
        ..session::Message::default()
    };
    if let Err(e) = session.broadcast(&msg) {
        session::diag::log_diag(&format!("{label}: state-delta broadcast failed: {e}"));
    }
}

fn reactions_to_wire(aggregated: &[AggregatedReaction]) -> Vec<ReactionAggregateWire> {
    aggregated
        .iter()
        .map(|a| ReactionAggregateWire {
            message_key: a.message_key.clone(),
            emoji: a.emoji.clone(),
            count: a.count,
            users: a.users.clone(),
        })
        .collect()
}

fn ci_to_wire(snap: &CiSnapshot) -> CiSnapshotWire {
    CiSnapshotWire {
        state: snap.state,
        run_id: snap.run_id,
        head_sha: snap.head_sha.clone(),
        display_title: snap.display_title.clone(),
    }
}

fn roster_to_wire(members: &[RosterMember]) -> Vec<RosterMemberWire> {
    members
        .iter()
        .map(|m| RosterMemberWire {
            routing_id: m.routing_id.clone(),
            name: m.name.clone(),
            identity: m.identity.clone(),
            role: m.role,
            presence: m.presence,
        })
        .collect()
}

pub struct ReactionsSessionBroadcaster {
    session: Arc<session::Session>,
}

impl ReactionsSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl ReactionsBroadcastListener for ReactionsSessionBroadcaster {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]) {
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Reactions {
                channel: channel.to_string(),
                aggregated: reactions_to_wire(aggregated),
            },
            "reactions_listener",
        );
    }
}

pub struct RosterSessionBroadcaster {
    session: Arc<session::Session>,
}

impl RosterSessionBroadcaster {
    pub fn new(session: Arc<session::Session>) -> Self {
        Self { session }
    }
}

impl RosterBroadcastListener for RosterSessionBroadcaster {
    fn on_changed(&self, members: &[RosterMember]) {
        broadcast_state_delta(
            &self.session,
            StateDeltaPayload::Roster {
                members: roster_to_wire(members),
            },
            "roster_listener",
        );
    }
}

fn ready_queue_is_empty(stories: &[Story]) -> bool {
    let current = current_iteration_number(stories);
    !stories.iter().any(|s| {
        s.state == StoryState::Ready
            && match (current, s.iteration_membership) {
                (Some(ci), Some((it, _))) => it == ci,
                (None, _) => true,
                _ => false,
            }
    })
}

fn any_owner_parked_delivery(stories: &[Story]) -> bool {
    stories
        .iter()
        .any(|s| s.state == StoryState::Delivered && s.acceptor == Some(Acceptor::Owner))
}

pub fn owner_park_presence(
    stories: &[Story],
    roster: &[RosterMember],
    delivering_role: Option<&str>,
) -> Vec<(String, Presence)> {
    let parked = any_owner_parked_delivery(stories) && ready_queue_is_empty(stories);
    let mut changes = Vec::new();
    for member in roster {
        let should_block = parked
            && (Some(member.name.as_str()) == delivering_role
                || member.presence == Presence::Blocked);
        let next = if should_block {
            Presence::Blocked
        } else if member.presence == Presence::Blocked {
            Presence::Online
        } else {
            continue;
        };
        if next != member.presence {
            changes.push((member.routing_id.clone(), next));
        }
    }
    changes
}

pub struct OwnerParkBroadcaster<T: CiPollFetch> {
    orchestrator: Arc<Orchestrator<T>>,
    store: Arc<BoardStore>,
}

impl<T: CiPollFetch> OwnerParkBroadcaster<T> {
    pub fn new(orchestrator: Arc<Orchestrator<T>>, store: Arc<BoardStore>) -> Self {
        Self {
            orchestrator,
            store,
        }
    }

    fn delivering_role(event: &BoardEvent) -> Option<String> {
        match event {
            BoardEvent::StateTransitioned {
                meta,
                new_state: StoryState::Delivered,
                ..
            } => Some(crate::transport::ws::short_name(&meta.actor).to_string()),
            _ => None,
        }
    }

    async fn dispatch_changes(&self, delivering: Option<&str>) {
        let stories = self.store.all_stories();
        let roster = self.orchestrator.roster_store().members();
        let changes = owner_park_presence(&stories, &roster, delivering);
        for (routing_id, presence) in changes {
            self.orchestrator
                .dispatch(OrchestratorAction::Roster(
                    RosterAction::RosterPresenceChanged {
                        routing_id,
                        presence,
                    },
                ))
                .await;
        }
    }

    pub async fn on_board_event(&self, event: &BoardEvent) {
        let delivering = Self::delivering_role(event);
        self.dispatch_changes(delivering.as_deref()).await;
    }

    pub async fn recompute_from_snapshot(&self) {
        self.dispatch_changes(None).await;
    }
}
