#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::Mutex;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::ci::{Orchestrator, OrchestratorAction};
use super::stores::{
    AggregatedReaction, PinnedMessage, PinsAction, PinsBroadcastListener, Presence,
    ReactionsAction, ReactionsBroadcastListener, RosterAction, RosterBroadcastListener,
    RosterMember, RosterRole,
};
use crate::board::adapters::github::GitHubSourceAdapter;
use crate::session::{Message, Session};
use crate::transport::ci_state_store::CiStateStore;

fn fresh_orch() -> Arc<Orchestrator<GitHubSourceAdapter>> {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "local".to_string(),
        tx,
    ));
    std::mem::forget(dir);
    Arc::new(Orchestrator::new(
        session,
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ))
}

#[derive(Default)]
struct RecReactions {
    received: Mutex<Vec<(String, Vec<AggregatedReaction>)>>,
}
impl ReactionsBroadcastListener for RecReactions {
    fn on_changed(&self, channel: &str, aggregated: &[AggregatedReaction]) {
        let mut r = self.received.lock().unwrap_or_else(|e| e.into_inner());
        r.push((channel.to_string(), aggregated.to_vec()));
    }
}

#[derive(Default)]
struct RecPins {
    received: Mutex<Vec<(String, Vec<PinnedMessage>)>>,
}
impl PinsBroadcastListener for RecPins {
    fn on_changed(&self, channel: &str, pins: &[PinnedMessage]) {
        let mut r = self.received.lock().unwrap_or_else(|e| e.into_inner());
        r.push((channel.to_string(), pins.to_vec()));
    }
}

#[derive(Default)]
struct RecRoster {
    received: Mutex<Vec<Vec<RosterMember>>>,
}
impl RosterBroadcastListener for RecRoster {
    fn on_changed(&self, members: &[RosterMember]) {
        let mut r = self.received.lock().unwrap_or_else(|e| e.into_inner());
        r.push(members.to_vec());
    }
}

#[tokio::test]
async fn reactions_action_drives_reducer_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Reactions(
        ReactionsAction::ReactionToggled {
            channel: "broadcast".to_string(),
            message_key: "k1".to_string(),
            emoji: "👍".to_string(),
            user: "ryan".to_string(),
            ts_iso: "t0".to_string(),
        },
    ))
    .await;

    let agg = orch.reactions_store().aggregated_for_channel("broadcast");
    assert_eq!(agg.len(), 1);
    assert_eq!(agg[0].count, 1);
    assert_eq!(agg[0].users, vec!["ryan".to_string()]);

    let received = rec.received.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].0, "broadcast");
    assert_eq!(received[0].1.len(), 1);
}

#[tokio::test]
async fn reactions_toggle_off_clears_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    let toggle = OrchestratorAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: "ryan".to_string(),
        ts_iso: "t0".to_string(),
    });
    orch.dispatch(toggle.clone()).await;
    orch.dispatch(toggle).await;

    let agg = orch.reactions_store().aggregated_for_channel("broadcast");
    assert!(agg.is_empty(), "second toggle removes the reaction");

    let received = rec.received.lock().unwrap();
    assert_eq!(received.len(), 2, "both add + remove fire listener");
}

#[tokio::test]
async fn reactions_aggregates_count_per_emoji_not_raw_events() {
    let orch = fresh_orch();
    let toggle_user = |user: &str| {
        OrchestratorAction::Reactions(ReactionsAction::ReactionToggled {
            channel: "broadcast".to_string(),
            message_key: "k1".to_string(),
            emoji: "👍".to_string(),
            user: user.to_string(),
            ts_iso: "t".to_string(),
        })
    };
    orch.dispatch(toggle_user("ryan")).await;
    orch.dispatch(toggle_user("dev")).await;
    orch.dispatch(toggle_user("designer")).await;

    let agg = orch.reactions_store().aggregated_for_channel("broadcast");
    assert_eq!(agg.len(), 1, "single emoji bucket aggregates 3 users");
    assert_eq!(agg[0].count, 3);
}

#[tokio::test]
async fn pins_added_drives_reducer_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecPins::default());
    orch.add_pins_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Pins(PinsAction::PinAdded {
        channel: "broadcast".to_string(),
        pin: PinnedMessage {
            message_key: "k1".to_string(),
            pinner: "ryan".to_string(),
            ts: "t0".to_string(),
            text: "pinned text".to_string(),
            from: "ryan".to_string(),
            pinned_at: "2026-05-08T00:00:00Z".to_string(),
        },
    }))
    .await;

    let pins = orch.pins_store().list_for_channel("broadcast");
    assert_eq!(pins.len(), 1);
    assert_eq!(pins[0].message_key, "k1");

    let received = rec.received.lock().unwrap();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].0, "broadcast");
}

#[tokio::test]
async fn pin_added_idempotent_dedup_no_listener_on_duplicate() {
    let orch = fresh_orch();
    let rec = Arc::new(RecPins::default());
    orch.add_pins_listener(rec.clone());

    let add = OrchestratorAction::Pins(PinsAction::PinAdded {
        channel: "broadcast".to_string(),
        pin: PinnedMessage {
            message_key: "k1".to_string(),
            pinner: "ryan".to_string(),
            ts: "t0".to_string(),
            text: "pinned text".to_string(),
            from: "ryan".to_string(),
            pinned_at: "2026-05-08T00:00:00Z".to_string(),
        },
    });
    orch.dispatch(add.clone()).await;
    orch.dispatch(add).await;

    let received = rec.received.lock().unwrap();
    assert_eq!(
        received.len(),
        1,
        "duplicate add is reducer-Unchanged → no listener fire"
    );
}

#[tokio::test]
async fn pin_remove_clears_channel_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecPins::default());
    orch.add_pins_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Pins(PinsAction::PinAdded {
        channel: "broadcast".to_string(),
        pin: PinnedMessage {
            message_key: "k1".to_string(),
            pinner: "ryan".to_string(),
            ts: "t0".to_string(),
            text: "pinned text".to_string(),
            from: "ryan".to_string(),
            pinned_at: "2026-05-08T00:00:00Z".to_string(),
        },
    }))
    .await;
    orch.dispatch(OrchestratorAction::Pins(PinsAction::PinRemoved {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
    }))
    .await;

    let pins = orch.pins_store().list_for_channel("broadcast");
    assert!(pins.is_empty());
    let received = rec.received.lock().unwrap();
    assert_eq!(received.len(), 2);
}

#[tokio::test]
async fn roster_member_upserted_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberUpserted {
            name: "ryan".to_string(),
            identity: "ryan:electron".to_string(),
            role: RosterRole::Owner,
            presence: Presence::Online,
        },
    ))
    .await;

    let members = orch.roster_store().members();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].role, RosterRole::Owner);
    assert_eq!(members[0].presence, Presence::Online);

    let received = rec.received.lock().unwrap();
    assert_eq!(received.len(), 1);
}

#[tokio::test]
async fn roster_presence_change_fires_only_when_actually_changed() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberUpserted {
            name: "dev".to_string(),
            identity: "dev:claude-code".to_string(),
            role: RosterRole::Dev,
            presence: Presence::Online,
        },
    ))
    .await;
    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterPresenceChanged {
            name: "dev".to_string(),
            presence: Presence::Online,
        },
    ))
    .await;
    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterPresenceChanged {
            name: "dev".to_string(),
            presence: Presence::Busy,
        },
    ))
    .await;

    let received = rec.received.lock().unwrap();
    assert_eq!(
        received.len(),
        2,
        "upsert + transition fire; same-presence-no-op doesn't fire"
    );
    assert_eq!(received[1][0].presence, Presence::Busy);
}

#[tokio::test]
async fn post_reaction_rest_handler_dispatches_through_orchestrator() {
    use crate::transport::rest::{ReactionToggle, post_reaction};
    use axum::body::{self, Body};
    use axum::http::Request;
    use tower::util::ServiceExt;

    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let orch: Arc<Orchestrator<GitHubSourceAdapter>> = Arc::new(Orchestrator::new(
        session.clone(),
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .layer(axum::Extension(orch.clone()))
        .with_state(session.clone());

    let body_text = serde_json::to_string(&serde_json::json!({
        "channel": "broadcast",
        "message_key": "k1",
        "emoji": "👍",
        "user": "ryan"
    }))
    .unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/reactions")
                .header("Content-Type", "application/json")
                .body(Body::from(body_text))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let _ = body::to_bytes(response.into_body(), 4096).await.unwrap();
    let _ = ReactionToggle {
        channel: "_unused".to_string(),
        message_key: "_unused".to_string(),
        emoji: "_unused".to_string(),
        user: "_unused".to_string(),
    };

    let agg = orch.reactions_store().aggregated_for_channel("broadcast");
    assert_eq!(
        agg.len(),
        1,
        "REST POST mutated orchestrator's ReactionsStore via dispatch"
    );
    assert_eq!(agg[0].count, 1);

    let received = rec.received.lock().unwrap();
    assert_eq!(
        received.len(),
        1,
        "REST POST → orchestrator.dispatch → reducer Changed → listener fired"
    );
    assert_eq!(received[0].0, "broadcast");
}

#[tokio::test]
async fn post_pin_rest_handler_dispatches_through_orchestrator() {
    use crate::transport::rest::post_pin;
    use axum::body::{self, Body};
    use axum::http::Request;
    use tower::util::ServiceExt;

    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "test".to_string(),
        tx,
    ));
    let orch: Arc<Orchestrator<GitHubSourceAdapter>> = Arc::new(Orchestrator::new(
        session.clone(),
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let rec = Arc::new(RecPins::default());
    orch.add_pins_listener(rec.clone());

    let router = axum::Router::new()
        .route("/api/pins", axum::routing::post(post_pin))
        .layer(axum::Extension(orch.clone()))
        .with_state(session.clone());

    let body_text = serde_json::to_string(&serde_json::json!({
        "channel": "broadcast",
        "message_key": "k1",
        "from": "dev:claude-code",
        "ts": "2026-04-22T12:00:00Z",
        "text": "pinned message body",
        "pinned_by": "ryan"
    }))
    .unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/pins")
                .header("Content-Type", "application/json")
                .body(Body::from(body_text))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let _ = body::to_bytes(response.into_body(), 4096).await.unwrap();

    let pins = orch.pins_store().list_for_channel("broadcast");
    assert_eq!(
        pins.len(),
        1,
        "REST POST mutated orchestrator's PinsStore via dispatch"
    );
    assert_eq!(pins[0].message_key, "k1");

    let received = rec.received.lock().unwrap();
    assert_eq!(
        received.len(),
        1,
        "REST POST → orchestrator.dispatch → reducer Changed → listener fired"
    );
}

#[tokio::test]
async fn roster_remove_fires_listener_when_member_existed() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberUpserted {
            name: "designer".to_string(),
            identity: "designer:claude-code".to_string(),
            role: RosterRole::Designer,
            presence: Presence::Online,
        },
    ))
    .await;
    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberRemoved {
            name: "designer".to_string(),
        },
    ))
    .await;
    orch.dispatch(OrchestratorAction::Roster(
        RosterAction::RosterMemberRemoved {
            name: "designer".to_string(),
        },
    ))
    .await;

    assert!(orch.roster_store().members().is_empty());
    let received = rec.received.lock().unwrap();
    assert_eq!(
        received.len(),
        2,
        "upsert + remove fire; second remove of absent member is no-op"
    );
}
