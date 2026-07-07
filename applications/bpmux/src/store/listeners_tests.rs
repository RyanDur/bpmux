#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use super::app::OwnerParkListener;
use super::app::{self, AppAction, AppState, RosterBroadcast};
use super::{
    AggregatedReaction, CiSessionBroadcaster, Presence, ReactionsBroadcastListener,
    ReactionsSessionBroadcaster, RosterBroadcastListener, RosterMember, RosterRole,
    RosterSessionBroadcaster,
};
use crate::ci::CiState;
use crate::kernel;
use crate::message::{Message, MessageType, StateDeltaPayload};
use crate::product_management::story::slice::StoryAction;
use crate::session::Session;
use crate::store::CiBroadcastListener;
use crate::store::ci::CiSnapshot;

fn owner_park_kernel(session: Arc<Session>) -> Arc<kernel::Store<AppState, AppAction>> {
    let store = kernel::Store::new(AppState::default(), Box::new(app::app_reduce));
    let roster_listeners: app::RosterListeners =
        Arc::new(std::sync::Mutex::new(vec![
            Arc::new(RosterSessionBroadcaster::new(session)) as Arc<dyn RosterBroadcastListener>,
        ]));
    store.add_listener(Arc::new(RosterBroadcast::new(roster_listeners)));
    store.add_listener(Arc::new(OwnerParkListener));
    store
}

fn seed_dev_online_kernel(store: &Arc<kernel::Store<AppState, AppAction>>) {
    store.dispatch(AppAction::Roster(
        crate::store::RosterAction::RosterMemberUpserted {
            routing_id: "sess-dev".to_string(),
            name: "dev".to_string(),
            identity: "dev:claude-code".to_string(),
            role: RosterRole::Dev,
            presence: Presence::Online,
        },
    ));
}

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
            assert_eq!(snapshot.state, CiState::Green.into());
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

fn owner_parked_story(id: u64) -> crate::product_management::story::Story {
    use crate::product_management::story::{StoryId, Work};
    let mut story = crate::product_management::story::Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        Work::Acceptance,
        chrono::Utc::now(),
        "dev".to_string(),
    );
    story.acceptor = Some(crate::product_management::story::Acceptor::Owner);
    story
}

fn ready_story(id: u64) -> crate::product_management::story::Story {
    use crate::product_management::story::{StoryId, Work};
    crate::product_management::story::Story::new_minimal(
        StoryId::new(id),
        format!("ready {id}"),
        Work::NotStarted,
        chrono::Utc::now(),
        "pm".to_string(),
    )
}

fn current_iteration(
    id: u64,
    story_ids: &[u64],
) -> crate::product_management::iteration::Iteration {
    crate::product_management::iteration::Iteration {
        id: crate::product_management::iteration::IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: chrono::Utc::now(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: story_ids
            .iter()
            .map(|n| crate::product_management::story::StoryId::new(*n))
            .collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }
}

fn state_transitioned(
    id: u64,
    actor: &str,
    new_state: crate::product_management::story::Work,
) -> crate::product_management::story::events::StoryEvent {
    crate::product_management::story::events::StoryEvent::StateTransitioned {
        meta: crate::product_management::story::events::EventMeta {
            actor: actor.to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::story::peer::PeerId::local(),
        },
        id: crate::product_management::story::StoryId::new(id),
        new_state,
        placement: None,
    }
}

async fn drain_roster_presence(
    rx: &mut broadcast::Receiver<Message>,
) -> Vec<crate::message::RosterMemberWire> {
    loop {
        let msg = rx
            .recv()
            .await
            .expect("expected a roster state-delta broadcast");
        if msg.message_type != MessageType::StateDelta {
            continue;
        }
        if let Some(StateDeltaPayload::Roster { members }) = msg.state_delta {
            return members;
        }
    }
}

#[tokio::test]
async fn owner_park_marks_delivering_role_blocked_and_broadcasts() {
    use crate::product_management::story::Work;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    let store = owner_park_kernel(session);
    seed_dev_online_kernel(&store);
    let _seed = drain_roster_presence(&mut rx).await;

    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(900),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Event(state_transitioned(
        900,
        "dev",
        Work::Acceptance,
    ))));

    let members = drain_roster_presence(&mut rx).await;
    let dev = members
        .iter()
        .find(|m| m.name == "dev")
        .expect("dev in roster");
    assert_eq!(
        dev.presence,
        crate::message::PresenceWire::Blocked,
        "delivering role becomes Blocked when its story is owner-parked with empty ready queue"
    );
}

#[tokio::test]
async fn owner_park_recomputes_to_online_when_ready_queue_refills() {
    use crate::product_management::story::Work;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    let store = owner_park_kernel(session);
    seed_dev_online_kernel(&store);
    store.dispatch(AppAction::Iteration(
        crate::product_management::iteration::IterationAction::Upserted(vec![current_iteration(
            800,
            &[901],
        )]),
    ));
    let _seed = drain_roster_presence(&mut rx).await;

    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(900),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Event(state_transitioned(
        900,
        "dev",
        Work::Acceptance,
    ))));
    let blocked = drain_roster_presence(&mut rx).await;
    assert_eq!(
        blocked
            .iter()
            .find(|m| m.name == "dev")
            .expect("dev")
            .presence,
        crate::message::PresenceWire::Blocked
    );
    while rx.try_recv().is_ok() {}

    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(900),
        ready_story(901),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Event(state_transitioned(
        901,
        "pm",
        Work::NotStarted,
    ))));

    let members = drain_roster_presence(&mut rx).await;
    let dev_presence = members
        .iter()
        .find(|m| m.name == "dev")
        .expect("dev")
        .presence;
    assert_eq!(
        dev_presence,
        crate::message::PresenceWire::Online,
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
            assert_eq!(members[0].role, crate::message::RosterRoleWire::Owner);
            assert_eq!(members[0].presence, crate::message::PresenceWire::Online);
        }
        _ => panic!("expected Roster variant"),
    }
}
