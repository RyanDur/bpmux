#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use super::listeners::{
    CiSessionBroadcaster, PinsSessionBroadcaster, ReactionsSessionBroadcaster,
    RosterSessionBroadcaster,
};
use super::stores::{
    AggregatedReaction, PinnedMessage, PinsBroadcastListener, Presence, ReactionsBroadcastListener,
    RosterBroadcastListener, RosterMember, RosterRole,
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
        users: vec!["ryan".to_string(), "dev".to_string()],
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

#[tokio::test]
async fn pins_listener_emits_typed_state_delta() {
    let (session, mut rx, _dir) = fresh_session();
    let listener = PinsSessionBroadcaster::new(session);
    let pins = vec![PinnedMessage {
        message_key: "m1".to_string(),
        pinner: "ryan".to_string(),
        ts: "2026-05-08T00:00:00Z".to_string(),
    }];
    listener.on_changed("broadcast", &pins);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Pins { channel, pins } => {
            assert_eq!(channel, "broadcast");
            assert_eq!(pins.len(), 1);
            assert_eq!(pins[0].message_key, "m1");
            assert_eq!(pins[0].pinner, "ryan");
        }
        _ => panic!("expected Pins variant"),
    }
}

#[tokio::test]
async fn roster_listener_emits_typed_state_delta() {
    let (session, mut rx, _dir) = fresh_session();
    let listener = RosterSessionBroadcaster::new(session);
    let members = vec![RosterMember {
        name: "ryan".to_string(),
        role: RosterRole::Owner,
        presence: Presence::Online,
    }];
    listener.on_changed(&members);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Roster { members } => {
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].name, "ryan");
            assert_eq!(members[0].role, RosterRole::Owner);
            assert_eq!(members[0].presence, Presence::Online);
        }
        _ => panic!("expected Roster variant"),
    }
}
