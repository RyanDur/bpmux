#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::sync::Mutex;

use tempfile::TempDir;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use super::test_support::eventually;
use super::{
    AggregatedReaction, AppAction, Presence, ReactionsAction, ReactionsBroadcastListener,
    RosterAction, RosterBroadcastListener, RosterMember, RosterRole, Store,
};
use crate::ci::CiStateStore;
use crate::hub::Hub;
use crate::server::store_wiring::build_store;
use message::Message;

fn fresh_orch() -> Arc<Store> {
    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "local".to_string(), tx));
    std::mem::forget(dir);
    Arc::new(build_store(
        hub,
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
async fn a_reactions_change_does_not_broadcast_roster() {
    let orch = fresh_orch();
    let roster_rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(roster_rec.clone());

    orch.dispatch(AppAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: "owner".to_string(),
        ts_iso: "t0".to_string(),
    }))
    .await;

    assert!(
        roster_rec.received.lock().unwrap().is_empty(),
        "a reactions change must not fire the roster broadcaster — slices are isolated"
    );
}

#[tokio::test]
async fn reactions_action_drives_reducer_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    orch.dispatch(AppAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: "owner".to_string(),
        ts_iso: "t0".to_string(),
    }))
    .await;

    let agg = orch.reactions_aggregated_for_channel("broadcast");
    assert_eq!(agg.len(), 1);
    assert_eq!(agg[0].count, 1);
    assert_eq!(agg[0].users, vec!["owner".to_string()]);

    eventually("reactions listener fired once", || {
        rec.received.lock().unwrap().len() == 1
    })
    .await;
    let received = rec.received.lock().unwrap();
    assert_eq!(received[0].0, "broadcast");
    assert_eq!(received[0].1.len(), 1);
}

#[tokio::test]
async fn a_distinct_channel_reaction_burst_broadcasts_once_per_channel_in_order() {
    let orch = fresh_orch();
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    for channel in ["alpha", "beta", "gamma"] {
        orch.dispatch(AppAction::Reactions(ReactionsAction::ReactionToggled {
            channel: channel.to_string(),
            message_key: "k1".to_string(),
            emoji: "👍".to_string(),
            user: "owner".to_string(),
            ts_iso: "t0".to_string(),
        }))
        .await;
    }

    eventually("every distinct-channel toggle broadcasts", || {
        rec.received.lock().unwrap().len() == 3
    })
    .await;
    let received = rec.received.lock().unwrap();
    let channels: Vec<String> = received
        .iter()
        .map(|(channel, _)| channel.clone())
        .collect();
    assert_eq!(
        channels,
        vec!["alpha", "beta", "gamma"],
        "ReactionsBroadcast is ORDERED: a synchronous burst across distinct channels delivers one broadcast per channel, in dispatch order — coalescing (LATEST-STATE) would drop the alpha and beta effects and leave only gamma"
    );
}

#[tokio::test]
async fn reactions_toggle_off_clears_and_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    let toggle = AppAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: "owner".to_string(),
        ts_iso: "t0".to_string(),
    });
    orch.dispatch(toggle.clone()).await;
    orch.dispatch(toggle).await;

    let agg = orch.reactions_aggregated_for_channel("broadcast");
    assert!(agg.is_empty(), "second toggle removes the reaction");

    eventually("both add + remove fire listener", || {
        rec.received.lock().unwrap().len() == 2
    })
    .await;
}

#[tokio::test]
async fn reactions_aggregates_count_per_emoji_not_raw_events() {
    let orch = fresh_orch();
    let toggle_user = |user: &str| {
        AppAction::Reactions(ReactionsAction::ReactionToggled {
            channel: "broadcast".to_string(),
            message_key: "k1".to_string(),
            emoji: "👍".to_string(),
            user: user.to_string(),
            ts_iso: "t".to_string(),
        })
    };
    orch.dispatch(toggle_user("owner")).await;
    orch.dispatch(toggle_user("dev")).await;
    orch.dispatch(toggle_user("designer")).await;

    let agg = orch.reactions_aggregated_for_channel("broadcast");
    assert_eq!(agg.len(), 1, "single emoji bucket aggregates 3 users");
    assert_eq!(agg[0].count, 3);
}

#[tokio::test]
async fn roster_wire_role_follows_the_registered_assignment_not_the_display() {
    let orch = fresh_orch();
    orch.dispatch(AppAction::Routing(
        crate::store::routing::RoutingAction::Registered {
            user_id: "sess-aria".to_string(),
            display: "aria".to_string(),
            role: Some("dev".to_string()),
        },
    ))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-aria".to_string(),
        identity: "aria".to_string(),
        presence: Presence::Online,
    }))
    .await;

    let snapshot = orch.build_snapshot();
    let member = snapshot
        .roster
        .iter()
        .find(|m| m.routing_id == "sess-aria")
        .expect("aria in roster wire projection");
    assert_eq!(
        member.role,
        message::RosterRoleWire::Dev,
        "the registered ASSIGNMENT (role=dev) drives the roster role even though the display \
         ('aria') carries no workflow-role prefix — pre-C3 the role was laundered from \
         short_name(display), which would derive Other here"
    );
}

#[tokio::test]
async fn roster_wire_role_falls_back_to_display_derivation_for_an_unassigned_member() {
    let orch = fresh_orch();
    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-legacy".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    }))
    .await;

    let snapshot = orch.build_snapshot();
    let member = snapshot
        .roster
        .iter()
        .find(|m| m.routing_id == "sess-legacy")
        .expect("legacy member in roster wire projection");
    assert_eq!(
        member.role,
        message::RosterRoleWire::Dev,
        "a member with NO recorded routing assignment (never registered) falls back to the \
         display-prefix derivation, unchanged from pre-C3 behavior"
    );
}

#[tokio::test]
async fn roster_member_upserted_fires_listener() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-owner".to_string(),
        identity: "owner:electron".to_string(),
        presence: Presence::Online,
    }))
    .await;

    let members = orch.roster_members();
    assert_eq!(members.len(), 1);
    assert_eq!(
        crate::roster::role_of(&members[0].identity),
        RosterRole::Owner
    );
    assert_eq!(members[0].presence, Presence::Online);

    eventually("roster upsert fires listener once", || {
        rec.received.lock().unwrap().len() == 1
    })
    .await;
}

#[tokio::test]
async fn roster_upsert_keys_by_routing_id_so_register_renames_the_default_in_place_no_dup() {
    let orch = fresh_orch();

    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-1".to_string(),
        identity: "bpmux-designer".to_string(),
        presence: Presence::Online,
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-1".to_string(),
        identity: "designer".to_string(),
        presence: Presence::Online,
    }))
    .await;

    let members = orch.roster_members();
    assert_eq!(
        members.len(),
        1,
        "the default and the set-display are ONE row (same routing-id), not two — the dup in owner's screenshot is structurally impossible; got {members:?}"
    );
    assert_eq!(
        crate::roster::display_name(&members[0].identity),
        "designer",
        "register REPLACES the dir-name default with the set display name in place"
    );
    assert_eq!(
        crate::roster::role_of(&members[0].identity),
        RosterRole::Designer
    );
}

#[tokio::test]
async fn roster_two_agents_sharing_a_display_are_two_rows_keyed_by_routing_id() {
    let orch = fresh_orch();
    for routing_id in ["sess-a", "sess-b"] {
        orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            identity: "dev".to_string(),
            presence: Presence::Online,
        }))
        .await;
    }

    let members = orch.roster_members();
    assert_eq!(
        members.len(),
        2,
        "two distinct connected agents that happen to share a display-name are TWO rows — the key is the routing-id, never the name (name-keying would collapse them into one); got {members:?}"
    );
}

#[tokio::test]
async fn roster_presence_change_targets_the_row_by_routing_id_not_a_shared_display() {
    let orch = fresh_orch();
    for routing_id in ["sess-a", "sess-b"] {
        orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
            routing_id: routing_id.to_string(),
            identity: "dev".to_string(),
            presence: Presence::Online,
        }))
        .await;
    }
    orch.dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
        routing_id: "sess-a".to_string(),
        presence: Presence::Busy,
    }))
    .await;

    let members = orch.roster_members();
    let by_id = |id: &str| {
        members
            .iter()
            .find(|m| m.routing_id == id)
            .unwrap()
            .presence
    };
    assert_eq!(
        by_id("sess-a"),
        Presence::Busy,
        "presence targets exactly the addressed routing-id"
    );
    assert_eq!(
        by_id("sess-b"),
        Presence::Online,
        "the sibling sharing the same display is untouched — presence is per-id, not per-name"
    );
}

#[tokio::test]
async fn roster_presence_change_fires_only_when_actually_changed() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-dev".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
        routing_id: "sess-dev".to_string(),
        presence: Presence::Online,
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
        routing_id: "sess-dev".to_string(),
        presence: Presence::Busy,
    }))
    .await;

    eventually(
        "upsert + transition fire; same-presence-no-op doesn't",
        || rec.received.lock().unwrap().len() == 2,
    )
    .await;
    let received = rec.received.lock().unwrap();
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
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "test".to_string(), tx));
    let orch: Arc<Store> = Arc::new(build_store(
        hub.clone(),
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));
    let rec = Arc::new(RecReactions::default());
    orch.add_reactions_listener(rec.clone());

    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::post(post_reaction))
        .layer(axum::Extension(orch.clone()))
        .with_state(hub.clone());

    let body_text = serde_json::to_string(&serde_json::json!({
        "channel": "broadcast",
        "message_key": "k1",
        "emoji": "👍",
        "user": "owner"
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

    let agg = orch.reactions_aggregated_for_channel("broadcast");
    assert_eq!(
        agg.len(),
        1,
        "REST POST mutated store's ReactionsStore via dispatch"
    );
    assert_eq!(agg[0].count, 1);

    eventually(
        "REST POST -> dispatch -> reducer Changed -> listener fired",
        || rec.received.lock().unwrap().len() == 1,
    )
    .await;
    let received = rec.received.lock().unwrap();
    assert_eq!(received[0].0, "broadcast");
}

#[tokio::test]
async fn get_reactions_reads_the_store_selector_with_no_file_present() {
    use crate::transport::rest::get_reactions;
    use axum::body::{self, Body};
    use axum::http::Request;
    use tower::util::ServiceExt;

    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "test".to_string(), tx));
    let orch: Arc<Store> = Arc::new(build_store(
        hub.clone(),
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));

    orch.dispatch(AppAction::Reactions(ReactionsAction::ReactionToggled {
        channel: "broadcast".to_string(),
        message_key: "k1".to_string(),
        emoji: "👍".to_string(),
        user: "owner".to_string(),
        ts_iso: "2026-07-08T00:00:00Z".to_string(),
    }))
    .await;

    assert!(
        !reactions::reactions_path(&hub.dir).exists(),
        "no persister is registered on this store, so the data lives only in the store"
    );

    let router = axum::Router::new()
        .route("/api/reactions", axum::routing::get(get_reactions))
        .layer(axum::Extension(orch.clone()));
    let response = router
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/reactions?channel=broadcast")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let bytes = body::to_bytes(response.into_body(), 4096).await.unwrap();
    let agg: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        agg.as_array().map(|a| a.len()),
        Some(1),
        "GET returns the store-held aggregate although reactions.json never existed"
    );
    assert_eq!(agg[0]["count"], 1);
    assert!(
        !reactions::reactions_path(&hub.dir).exists(),
        "GET must not read or materialize a reactions.json file",
    );
}

#[tokio::test]
async fn post_reaction_returns_present_from_the_post_dispatch_selector() {
    use crate::transport::rest::post_reaction;
    use axum::body::{self, Body};
    use axum::http::Request;
    use tower::util::ServiceExt;

    let dir = TempDir::new().expect("tempdir");
    let (tx, _rx) = broadcast::channel::<Message>(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "test".to_string(), tx));
    let orch: Arc<Store> = Arc::new(build_store(
        hub.clone(),
        None,
        CiStateStore::new(),
        CancellationToken::new(),
    ));

    let post = |orch: Arc<Store>, hub: Arc<Hub>| async move {
        let router = axum::Router::new()
            .route("/api/reactions", axum::routing::post(post_reaction))
            .layer(axum::Extension(orch))
            .with_state(hub);
        let body_text = serde_json::to_string(&serde_json::json!({
            "channel": "broadcast",
            "message_key": "k1",
            "emoji": "👍",
            "user": "owner"
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
        let bytes = body::to_bytes(response.into_body(), 4096).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        value["present"].as_bool().unwrap()
    };

    assert!(
        post(orch.clone(), hub.clone()).await,
        "first toggle reports present via the post-dispatch selector read"
    );
    assert!(
        !post(orch.clone(), hub.clone()).await,
        "second toggle reports absent via the post-dispatch selector read"
    );
}

#[tokio::test]
async fn ws_dispatch_roster_join_populates_store_with_identity() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    crate::transport::ws::dispatch_roster_join(&orch, "sess-dev", "dev:claude-code").await;

    let members = orch.roster_members();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].routing_id, "sess-dev");
    assert_eq!(crate::roster::display_name(&members[0].identity), "dev");
    assert_eq!(members[0].identity, "dev:claude-code");
    assert_eq!(
        crate::roster::role_of(&members[0].identity),
        RosterRole::Dev
    );
    assert_eq!(members[0].presence, Presence::Online);

    eventually("join dispatches -> listener fires", || {
        rec.received.lock().unwrap().len() == 1
    })
    .await;
}

#[tokio::test]
async fn ws_dispatch_roster_leave_removes_member() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    crate::transport::ws::dispatch_roster_join(&orch, "sess-designer", "designer:claude-code")
        .await;
    crate::transport::ws::dispatch_roster_leave(&orch, "sess-designer").await;

    let members = orch.roster_members();
    assert!(members.is_empty(), "leave removes member from store");

    eventually("join + leave both fire listeners", || {
        rec.received.lock().unwrap().len() == 2
    })
    .await;
}

#[tokio::test]
async fn ws_dispatch_roster_join_role_mapping_covers_known_short_names() {
    let cases = [
        ("owner:electron", "owner", RosterRole::Owner),
        ("dev:claude-code", "dev", RosterRole::Dev),
        ("pm:claude-code", "pm", RosterRole::Pm),
        ("designer:claude-code", "designer", RosterRole::Designer),
        (
            "agent-abc123:claude-code",
            "agent-abc123",
            RosterRole::Other,
        ),
    ];
    for (identity, expected_name, expected_role) in cases {
        let orch = fresh_orch();
        crate::transport::ws::dispatch_roster_join(&orch, "sess-x", identity).await;
        let members = orch.roster_members();
        assert_eq!(members.len(), 1, "identity {identity}: 1 member");
        assert_eq!(
            crate::roster::display_name(&members[0].identity),
            expected_name
        );
        assert_eq!(crate::roster::role_of(&members[0].identity), expected_role);
        assert_eq!(members[0].identity, identity);
    }
}

#[tokio::test]
async fn roster_remove_fires_listener_when_member_existed() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-designer".to_string(),
        identity: "designer:claude-code".to_string(),
        presence: Presence::Online,
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberRemoved {
        routing_id: "sess-designer".to_string(),
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberRemoved {
        routing_id: "sess-designer".to_string(),
    }))
    .await;

    assert!(orch.roster_members().is_empty());
    eventually(
        "upsert + remove fire; second remove of absent is no-op",
        || rec.received.lock().unwrap().len() == 2,
    )
    .await;
}

#[tokio::test]
async fn roster_presence_change_to_blocked_records_blocked_presence() {
    let orch = fresh_orch();
    let rec = Arc::new(RecRoster::default());
    orch.add_roster_listener(rec.clone());

    orch.dispatch(AppAction::Roster(RosterAction::RosterMemberUpserted {
        routing_id: "sess-dev".to_string(),
        identity: "dev:claude-code".to_string(),
        presence: Presence::Online,
    }))
    .await;
    orch.dispatch(AppAction::Roster(RosterAction::RosterPresenceChanged {
        routing_id: "sess-dev".to_string(),
        presence: Presence::Blocked,
    }))
    .await;

    let members = orch.roster_members();
    assert_eq!(members.len(), 1);
    assert_eq!(
        members[0].presence,
        Presence::Blocked,
        "owner-park presence is recorded as first-class Blocked, not ephemeral overlay-text"
    );

    eventually("upsert + presence change both fire", || {
        rec.received.lock().unwrap().len() == 2
    })
    .await;
    let received = rec.received.lock().unwrap();
    assert_eq!(received.last().unwrap()[0].presence, Presence::Blocked);
}

#[test]
fn blocked_roster_member_wire_serializes_blocked_presence() {
    use message::{PresenceWire, RosterMemberWire, RosterRoleWire};
    let wire = RosterMemberWire {
        routing_id: "sess-dev".to_string(),
        name: "dev".to_string(),
        identity: "dev:claude-code".to_string(),
        role: RosterRoleWire::Dev,
        presence: PresenceWire::Blocked,
    };
    let json = serde_json::to_string(&wire).unwrap();
    assert!(
        json.contains("\"presence\":\"Blocked\""),
        "Blocked rides the durable wire field, not Activity-text: {json}"
    );
}
