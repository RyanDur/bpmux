#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use tempfile::TempDir;
use tokio::sync::broadcast;

use ci::CiState;
use hub::Hub;
use message::{Message, MessageType, StateDeltaPayload};
use product_management::story::slice::StoryAction;
use store::CiBroadcastListener;
use store::ci::CiSnapshot;
use store::{
    AggregatedReaction, AppAction, AppState, BoardPlacementPersister, BoardPlacementReconciler,
    CiAction, CiBroadcast, CiListeners, OwnerParkListener, Presence, ReactionsBroadcastListener,
    RosterBroadcast, RosterBroadcastListener, RosterListeners, RosterMember, app_reduce,
};
use transport::state_delta_broadcasters::{
    CiHubBroadcaster, ReactionsHubBroadcaster, RosterHubBroadcaster,
};

fn owner_park_kernel(hub: Arc<Hub>) -> Arc<kernel::Store<AppState, AppAction>> {
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    let select = store::Select::new(Arc::downgrade(&store));
    let roster_listeners: RosterListeners =
        Arc::new(std::sync::Mutex::new(vec![
            Arc::new(RosterHubBroadcaster::new(hub, select)) as Arc<dyn RosterBroadcastListener>,
        ]));
    store.add_listener(Arc::new(RosterBroadcast::new(roster_listeners)));
    store.add_listener(Arc::new(OwnerParkListener));
    store
}

fn seed_dev_online_kernel(store: &Arc<kernel::Store<AppState, AppAction>>) {
    store.dispatch(AppAction::Roster(
        store::RosterAction::RosterMemberUpserted {
            routing_id: "sess-dev".to_string(),
            identity: "dev:claude-code".to_string(),
            presence: Presence::Online,
        },
    ));
}

fn fresh_hub() -> (Arc<Hub>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().expect("tempdir");
    let (tx, rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    (hub, rx, dir)
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

fn ci_snap(run_id: u64, conclusion: &'static str) -> CiSnapshot {
    CiSnapshot {
        state: CiState::Green,
        run_id: Some(run_id),
        head_sha: None,
        display_title: None,
        conclusion,
        workflows: Vec::new(),
    }
}

#[derive(Default)]
struct RecordingCiListener {
    received: std::sync::Mutex<Vec<CiSnapshot>>,
}

impl CiBroadcastListener for RecordingCiListener {
    fn on_changed(&self, snap: &CiSnapshot) {
        self.received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(snap.clone());
    }
}

#[tokio::test]
async fn ci_broadcast_fires_once_when_an_updated_lands_a_new_snapshot_key() {
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    let recorder = Arc::new(RecordingCiListener::default());
    let listeners: CiListeners = Arc::new(std::sync::Mutex::new(vec![
        recorder.clone() as Arc<dyn CiBroadcastListener>
    ]));
    store.add_listener(Arc::new(CiBroadcast::new(listeners)));

    store.dispatch(AppAction::Ci(CiAction::Updated(ci_snap(42, "success"))));

    crate::test_support::eventually("a new snapshot key still reaches the fan-out", || {
        recorder
            .received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
            == 1
    })
    .await;
}

#[tokio::test]
async fn ci_broadcast_ignores_a_ci_slice_change_that_keeps_the_snapshot_key() {
    use kernel::Listener;

    let recorder = Arc::new(RecordingCiListener::default());
    let listeners: CiListeners = Arc::new(std::sync::Mutex::new(vec![
        recorder.clone() as Arc<dyn CiBroadcastListener>
    ]));
    let broadcast = CiBroadcast::new(listeners);

    let seeded = store::ci::reduce(
        &store::ci::CiState::default(),
        &CiAction::Updated(ci_snap(42, "success")),
    )
    .expect("a first Updated lands the snapshot");
    let previous = Arc::new(AppState {
        ci: Arc::new(seeded.clone()),
        ..AppState::default()
    });
    let current = Arc::new(AppState {
        ci: Arc::new(seeded),
        ..AppState::default()
    });

    broadcast
        .on_action(kernel::Reaction {
            action: AppAction::Ci(CiAction::StartPolling),
            previous,
            current,
            dispatch: Arc::new(|_| {}),
        })
        .await;

    assert!(
        recorder
            .received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_empty(),
        "a ci-slice change that keeps the snapshot key must not re-broadcast — the trigger asks whether the snapshot changed, not whether the Arc changed"
    );
}

#[tokio::test]
async fn polling_liveness_facts_never_refire_the_ci_fan_out() {
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    let recorder = Arc::new(RecordingCiListener::default());
    let listeners: CiListeners = Arc::new(std::sync::Mutex::new(vec![
        recorder.clone() as Arc<dyn CiBroadcastListener>
    ]));
    store.add_listener(Arc::new(CiBroadcast::new(listeners)));

    store.dispatch(AppAction::Ci(CiAction::Updated(ci_snap(42, "success"))));
    crate::test_support::eventually("the seeded Updated reaches the fan-out once", || {
        recorder
            .received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len()
            == 1
    })
    .await;

    store.dispatch(AppAction::Ci(CiAction::PollingStarted));
    store.dispatch(AppAction::Ci(CiAction::PollingStopped));

    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    assert_eq!(
        recorder
            .received
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .len(),
        1,
        "a polling-liveness fact reports effector state, not a snapshot change — it must never re-fire the CI broadcast fan-out"
    );
}

#[tokio::test]
async fn ci_listener_emits_typed_state_delta() {
    let (hub, mut rx, _dir) = fresh_hub();
    let listener = CiHubBroadcaster::new(hub);
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
    let (hub, mut rx, _dir) = fresh_hub();
    let listener = ReactionsHubBroadcaster::new(hub);
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

fn owner_parked_story(id: u64) -> product_management::story::Story {
    use product_management::story::{StoryId, Work};
    let mut story = product_management::story::Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        Work::Acceptance,
        chrono::Utc::now(),
        "dev".to_string(),
    );
    story.acceptor = Some(product_management::story::Acceptor::Owner);
    story
}

fn ready_story(id: u64) -> product_management::story::Story {
    use product_management::story::{StoryId, Work};
    product_management::story::Story::new_minimal(
        StoryId::new(id),
        format!("ready {id}"),
        Work::NotStarted,
        chrono::Utc::now(),
        "pm".to_string(),
    )
}

fn current_iteration(id: u64, story_ids: &[u64]) -> product_management::iteration::Iteration {
    product_management::iteration::Iteration {
        id: product_management::iteration::IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: chrono::Utc::now(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: story_ids
            .iter()
            .map(|n| product_management::story::StoryId::new(*n))
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
    new_state: product_management::story::Work,
) -> product_management::story::events::StoryEvent {
    product_management::story::events::StoryEvent::new(
        product_management::story::events::EventMeta {
            actor: actor.to_string(),
            actor_id: None,
            ts: chrono::Utc::now(),
        },
        product_management::story::StoryId::new(id),
        product_management::story::events::StoryEventKind::StateTransitioned {
            new_state,
            placement: None,
        },
    )
}

fn state_transitioned_by_actor_id(
    id: u64,
    actor: &str,
    actor_id: &str,
    new_state: product_management::story::Work,
) -> product_management::story::events::StoryEvent {
    product_management::story::events::StoryEvent::new(
        product_management::story::events::EventMeta {
            actor: actor.to_string(),
            actor_id: Some(actor_id.to_string()),
            ts: chrono::Utc::now(),
        },
        product_management::story::StoryId::new(id),
        product_management::story::events::StoryEventKind::StateTransitioned {
            new_state,
            placement: None,
        },
    )
}

fn seed_dev_sibling_pair_online(store: &Arc<kernel::Store<AppState, AppAction>>) {
    store.dispatch(AppAction::Roster(
        store::RosterAction::RosterMemberUpserted {
            routing_id: "dev-routing-alpha".to_string(),
            identity: "dev:claude-code".to_string(),
            presence: Presence::Online,
        },
    ));
    store.dispatch(AppAction::Roster(
        store::RosterAction::RosterMemberUpserted {
            routing_id: "dev-routing-beta".to_string(),
            identity: "dev:vscode".to_string(),
            presence: Presence::Online,
        },
    ));
}

async fn drain_roster_presence(
    rx: &mut broadcast::Receiver<Message>,
) -> Vec<message::RosterMemberWire> {
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
async fn roster_changes_reach_the_wire_but_never_the_log() {
    let (hub, mut rx, _dir) = fresh_hub();
    let store = owner_park_kernel(hub.clone());

    seed_dev_online_kernel(&store);

    let members = drain_roster_presence(&mut rx).await;
    assert_eq!(members.len(), 1, "the wire carries the roster delta");
    let logged = hub.read_messages().expect("log readable");
    assert!(
        logged.is_empty(),
        "the log stays empty — roster rebuilds from live links, not history"
    );
}

#[tokio::test]
async fn owner_park_marks_delivering_role_blocked_and_broadcasts() {
    use product_management::story::Work;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    std::mem::forget(dir);
    let store = owner_park_kernel(hub);
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
        message::PresenceWire::Blocked,
        "delivering role becomes Blocked when its story is owner-parked with empty ready queue"
    );
}

fn presence_of(state: &AppState, routing_id: &str) -> Option<Presence> {
    store::roster::members(&state.roster)
        .into_iter()
        .find(|member| member.routing_id == routing_id)
        .map(|member| member.presence)
}

#[tokio::test]
async fn owner_park_blocks_only_the_delivering_id_never_a_same_display_sibling() {
    use product_management::story::Work;

    let (hub, _rx, _dir) = fresh_hub();
    let store = owner_park_kernel(hub);
    seed_dev_sibling_pair_online(&store);

    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(900),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Event(
        state_transitioned_by_actor_id(900, "dev", "dev-routing-alpha", Work::Acceptance),
    )));

    crate::test_support::eventually(
        "the delivering id (dev-routing-alpha) reaches Blocked while its same-display sibling \
         (dev-routing-beta) stays Online — matching on display_name instead of the delivering \
         actor_id would over-block dev-routing-beta too, the STOP-gated regression B-4 unblocks",
        || {
            store.select(|state| presence_of(state, "dev-routing-alpha")) == Some(Presence::Blocked)
                && store.select(|state| presence_of(state, "dev-routing-beta"))
                    == Some(Presence::Online)
        },
    )
    .await;
}

#[tokio::test]
async fn owner_park_recomputes_to_online_when_ready_queue_refills() {
    use product_management::story::Work;

    let dir = TempDir::new().expect("tempdir");
    let (tx, mut rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    std::mem::forget(dir);
    let store = owner_park_kernel(hub);
    seed_dev_online_kernel(&store);
    store.dispatch(AppAction::Iteration(
        product_management::iteration::IterationAction::Upserted(vec![current_iteration(
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
        message::PresenceWire::Blocked
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
        message::PresenceWire::Online,
        "presence recomputes to Online from durable state: the delivered story STILL awaits owner-accept, but a ready story appeared so the agent can pull again — no longer parked, never decayed-stale"
    );
}

fn dev_presence(state: &AppState) -> Option<Presence> {
    store::roster::members(&state.roster)
        .into_iter()
        .find(|member| roster::display_name(&member.identity) == "dev")
        .map(|member| member.presence)
}

#[tokio::test]
async fn owner_park_worker_reentrant_dispatch_lands_the_presence_change_in_durable_state() {
    use product_management::story::Work;

    let (hub, _rx, _dir) = fresh_hub();
    let store = owner_park_kernel(hub);
    seed_dev_online_kernel(&store);

    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(900),
    ])));
    store.dispatch(AppAction::Story(StoryAction::Event(state_transitioned(
        900,
        "dev",
        Work::Acceptance,
    ))));

    crate::test_support::eventually(
        "re-entrant presence change reaches durable roster state",
        || store.select(dev_presence) == Some(Presence::Blocked),
    )
    .await;
}

#[tokio::test]
async fn board_placement_reconciler_worker_reentrant_dispatch_unplaces_a_departed_story() {
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    store.add_listener(Arc::new(BoardPlacementReconciler));

    let story = product_management::story::StoryId::new(1);
    store.dispatch(AppAction::Board(board::BoardAction::StoryPlaced {
        story,
        stage: workflow::StageId::from("desk-check"),
    }));
    store.dispatch(AppAction::Story(StoryAction::Seeded(vec![
        owner_parked_story(1),
    ])));

    crate::test_support::eventually(
        "the reconciler worker re-dispatches StoryUnplaced and the board reflects it",
        || store.select(|state| board::placement_of(&state.board, story).is_none()),
    )
    .await;
}

#[tokio::test]
async fn a_board_placement_burst_persists_the_final_board_with_no_placement_lost() {
    let (hub, _rx, _dir) = fresh_hub();
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    store.add_listener(Arc::new(BoardPlacementPersister::new(hub.clone())));

    for id in [1u64, 2, 3] {
        store.dispatch(AppAction::Board(board::BoardAction::StoryPlaced {
            story: product_management::story::StoryId::new(id),
            stage: workflow::StageId::from("desk-check"),
        }));
    }

    let placed = |id: u64| {
        board::placement_of(
            &board::persistence::load(&hub.dir),
            product_management::story::StoryId::new(id),
        )
        .is_some()
    };
    crate::test_support::eventually("the coalesced placement burst is persisted", || placed(3))
        .await;
    assert!(
        placed(1) && placed(2) && placed(3),
        "the LATEST-STATE persister collapses the three-placement burst into one save yet persists ALL three placements — coalescing loses no change because it persists the current (final) board snapshot"
    );
}

#[tokio::test]
async fn roster_listener_emits_typed_state_delta() {
    let (hub, mut rx, _dir) = fresh_hub();
    let select = store::Select::new(std::sync::Weak::new());
    let listener = RosterHubBroadcaster::new(hub, select);
    let members = vec![RosterMember {
        routing_id: "sess-owner".to_string(),
        identity: "owner:electron".to_string(),
        presence: Presence::Online,
    }];
    listener.on_changed(&members);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Roster { members } => {
            assert_eq!(members.len(), 1);
            assert_eq!(members[0].name, "owner");
            assert_eq!(members[0].role, message::RosterRoleWire::Owner);
            assert_eq!(members[0].presence, message::PresenceWire::Online);
        }
        _ => panic!("expected Roster variant"),
    }
}

#[tokio::test]
async fn roster_listener_reports_the_registered_assignment_not_the_display() {
    let (hub, mut rx, _dir) = fresh_hub();
    let store = kernel::Store::new(AppState::default(), Box::new(app_reduce));
    let select = store::Select::new(Arc::downgrade(&store));
    let listener = RosterHubBroadcaster::new(hub, select);
    store.dispatch(AppAction::Routing(
        store::routing::RoutingAction::Registered {
            user_id: "sess-aria".to_string(),
            display: "aria".to_string(),
            role: Some("dev".to_string()),
        },
    ));
    let members = vec![RosterMember {
        routing_id: "sess-aria".to_string(),
        identity: "aria".to_string(),
        presence: Presence::Online,
    }];
    listener.on_changed(&members);
    let payload = drain_state_delta(&mut rx);
    match payload {
        StateDeltaPayload::Roster { members } => {
            assert_eq!(
                members[0].role,
                message::RosterRoleWire::Dev,
                "the broadcaster joins against the CURRENT routing assignment, not the display prefix"
            );
        }
        _ => panic!("expected Roster variant"),
    }
}
