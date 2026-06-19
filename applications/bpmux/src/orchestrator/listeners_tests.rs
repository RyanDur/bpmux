#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use super::listeners::{
    CiSessionBroadcaster, OwnerParkBroadcaster, ReactionsSessionBroadcaster,
    RosterSessionBroadcaster,
};
use super::stores::{
    AggregatedReaction, Presence, ReactionsBroadcastListener, RosterBroadcastListener,
    RosterMember, RosterRole,
};
use crate::ci::CiState;
use crate::orchestrator::CiBroadcastListener;
use crate::orchestrator::ci::CiSnapshot;
use crate::session::message::{MessageType, StateDeltaPayload};
use crate::session::{Message, Session};

fn fresh_session() -> (Arc<Session>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    (session, rx, dir)
}

fn drain_state_delta(rx: &mut broadcast::Receiver<Message>) -> StateDeltaPayload {
    loop {
        let msg = rx.try_recv().expect("expected at least one broadcast");
        if msg.message_type == MessageType::StateDelta {
            return msg
                .state_delta
                .expect("StateDelta message missing state_delta payload");
        }
    }
}

#[tokio::test]
async fn ci_listener_emits_typed_state_delta() {
    let (session, mut rx, _dir) = fresh_session();
    let listener = CiSessionBroadcaster::new(session);
    let snap = CiSnapshot {
        state: CiState::Green,
        run_id: Some(7),
        head_sha: Some("abc123".to_string()),
        display_title: Some("title".to_string()),
        conclusion: "success",
        workflows: Vec::new(),
    };
    listener.on_changed(&snap);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Ci { snapshot } => {
            assert_eq!(snapshot.state, CiState::Green);
            assert_eq!(snapshot.run_id, Some(7));
            assert_eq!(snapshot.head_sha.as_deref(), Some("abc123"));
        }
        _ => panic!("expected Ci variant"),
    }
}

#[tokio::test]
async fn reactions_listener_emits_typed_state_delta() {
    let (session, mut rx, _dir) = fresh_session();
    let listener = ReactionsSessionBroadcaster::new(session);
    let agg = vec![AggregatedReaction {
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        count: 2,
        users: vec!["owner".to_string(), "dev".to_string()],
    }];
    listener.on_changed("broadcast", &agg);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Reactions {
            channel,
            aggregated,
        } => {
            assert_eq!(channel, "broadcast");
            assert_eq!(aggregated.len(), 1);
            assert_eq!(aggregated[0].count, 2);
            assert_eq!(aggregated[0].emoji, "👍");
            assert_eq!(aggregated[0].users.len(), 2);
        }
        _ => panic!("expected Reactions variant"),
    }
}

fn owner_parked_story(number: u64) -> crate::board::Story {
    use crate::board::domain::{Acceptor, StoryState};
    let mut story = crate::board::Story::new_minimal(
        number,
        format!("story {number}"),
        StoryState::Delivered,
        chrono::Utc::now(),
        "dev".to_string(),
    );
    story.acceptor = Some(Acceptor::Owner);
    story
}

fn ready_story(number: u64, iteration: u64, position: u32) -> crate::board::Story {
    use crate::board::domain::StoryState;
    let mut story = crate::board::Story::new_minimal(
        number,
        format!("ready {number}"),
        StoryState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    );
    story.iteration_membership = Some((iteration, position));
    story
}

fn state_transitioned(
    number: u64,
    actor: &str,
    new_state: crate::board::StoryState,
) -> crate::board::BoardEvent {
    crate::board::BoardEvent::StateTransitioned {
        meta: crate::board::events::EventMeta {
            actor: actor.to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::peer::PeerId::local(),
        },
        number,
        new_state,
    }
}

fn drain_roster_presence(rx: &mut broadcast::Receiver<Message>) -> Vec<RosterMember> {
    loop {
        let msg = rx
            .try_recv()
            .expect("expected a roster state-delta broadcast");
        if msg.message_type != MessageType::StateDelta {
            continue;
        }
        if let Some(StateDeltaPayload::Roster { members }) = msg.state_delta {
            return members
                .into_iter()
                .map(|m| RosterMember {
                    routing_id: m.routing_id,
                    name: m.name,
                    identity: m.identity,
                    role: m.role,
                    presence: m.presence,
                })
                .collect();
        }
    }
}

async fn seed_dev_online(
    orch: &Arc<
        crate::orchestrator::ci::Orchestrator<crate::board::adapters::github::GitHubSourceAdapter>,
    >,
) {
    use crate::orchestrator::ci::OrchestratorAction;
    use crate::orchestrator::stores::RosterAction;
    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberUpserted {
            routing_id: "sess-dev".to_string(),
            name: "dev".to_string(),
            identity: "dev:claude-code".to_string(),
            role: RosterRole::Dev,
            presence: Presence::Online,
        },
    ))
    .await;
}

#[tokio::test]
async fn owner_park_marks_delivering_role_blocked_and_broadcasts() {
    use crate::board::BoardStore;
    use crate::board::StoryState;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    let orch = Arc::new(crate::orchestrator::ci::Orchestrator::new(
        session.clone(),
        None,
        crate::transport::ci_state_store::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ));
    orch.add_roster_listener(Arc::new(RosterSessionBroadcaster::new(session.clone())));
    seed_dev_online(&orch).await;
    while rx.try_recv().is_ok() {}

    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![owner_parked_story(900)]);

    let broadcaster = OwnerParkBroadcaster::new(orch.clone(), store.clone());
    broadcaster
        .on_board_event(&state_transitioned(900, "dev", StoryState::Delivered))
        .await;

    let members = drain_roster_presence(&mut rx);
    let dev = members
        .iter()
        .find(|m| m.name == "dev")
        .expect("dev in roster");
    assert_eq!(
        dev.presence,
        Presence::Blocked,
        "delivering role becomes Blocked when its story is owner-parked with empty ready queue"
    );
}

#[tokio::test]
async fn owner_park_recomputes_to_online_when_ready_queue_refills() {
    use crate::board::BoardStore;
    use crate::board::StoryState;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    let orch = Arc::new(crate::orchestrator::ci::Orchestrator::new(
        session.clone(),
        None,
        crate::transport::ci_state_store::CiStateStore::new(),
        tokio_util::sync::CancellationToken::new(),
    ));
    orch.add_roster_listener(Arc::new(RosterSessionBroadcaster::new(session.clone())));
    seed_dev_online(&orch).await;

    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![owner_parked_story(900)]);
    let broadcaster = OwnerParkBroadcaster::new(orch.clone(), store.clone());
    broadcaster
        .on_board_event(&state_transitioned(900, "dev", StoryState::Delivered))
        .await;
    assert_eq!(
        orch.roster_store()
            .members()
            .iter()
            .find(|m| m.name == "dev")
            .expect("dev")
            .presence,
        Presence::Blocked
    );
    while rx.try_recv().is_ok() {}

    store.seed_all(vec![owner_parked_story(900), ready_story(901, 800, 1)]);
    broadcaster
        .on_board_event(&state_transitioned(901, "pm", StoryState::Ready))
        .await;

    let dev_presence = orch
        .roster_store()
        .members()
        .iter()
        .find(|m| m.name == "dev")
        .expect("dev")
        .presence;
    assert_eq!(
        dev_presence,
        Presence::Online,
        "presence recomputes to Online from durable state: the delivered story STILL awaits owner-accept, but a ready story appeared so the agent can pull again — no longer parked, never decayed-stale"
    );
}

#[tokio::test]
async fn roster_listener_emits_typed_state_delta() {
    let (session, mut rx, _dir) = fresh_session();
    let listener = RosterSessionBroadcaster::new(session);
    let members = vec![RosterMember {
        routing_id: "sess-owner".to_string(),
        name: "owner".to_string(),
        identity: "owner:electron".to_string(),
        role: RosterRole::Owner,
        presence: Presence::Online,
    }];
    listener.on_changed(&members);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Roster { members } => {
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].name, "owner");
            assert_eq!(members[0].role, RosterRole::Owner);
            assert_eq!(members[0].presence, Presence::Online);
        }
        _ => panic!("expected Roster variant"),
    }
}
