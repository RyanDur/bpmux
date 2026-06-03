#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::create_target_story;
use serde_json::Value;

async fn drain_pending_events(agent: &TestBridge, subscription_id: &str) {
    loop {
        let drained = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 500 }),
            )
            .await
            .expect("drain wait_for_event");
        if drained["timed_out"].as_bool() == Some(true) {
            break;
        }
    }
}

async fn move_to_ready(mutator: &TestBridge, number: u64) {
    mutator
        .call_tool(
            "move_story",
            serde_json::json!({
                "number": number,
                "target_state": "ready",
                "rationale": "promote for the story_state_transition coverage test"
            }),
        )
        .await
        .expect("move_story succeeds");
}

#[test]
fn story_id_filter_discriminates_matching_from_non_matching_transition() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mutator = TestBridge::connect(&bpmux, "pm").await;

        let other_story = create_target_story(
            &mutator,
            "[Feature] story-transition-id-nonmatch",
            "a transition on this story must NOT reach a subscriber filtered to a different story_id",
        )
        .await;
        let story = create_target_story(
            &mutator,
            "[Feature] story-transition-id-target",
            "a transition on this story must reach the matching-story_id subscriber",
        )
        .await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "story_state_transition",
                    "filter": { "story_id": story }
                }),
            )
            .await
            .expect("subscribe (matching story_id) returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        move_to_ready(&mutator, other_story).await;
        move_to_ready(&mutator, story).await;

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("wait_for_event delivers the matching transition, not a timeout");

        assert_eq!(
            event["event_class"].as_str(),
            Some("story_state_transition"),
            "the matching subscriber must deliver a story_state_transition event; got {event}"
        );
        assert_eq!(
            event["payload"]["story_id"].as_u64(),
            Some(story),
            "the FIRST received event must carry the matching story's id — discrimination proof under any load: the other story's transition was driven FIRST, so were the story_id-filter to leak it the order-preserved dispatch would deliver the other story FIRST and this assert would read its id (no negative-timeout to outrun under CPU-starvation); got {event}"
        );
        assert_eq!(
            event["payload"]["from_state"].as_str(),
            Some("icebox"),
            "a freshly-created story starts in icebox; from_state must reflect the PRIOR state; got {event}"
        );
        assert_eq!(
            event["payload"]["target_state"].as_str(),
            Some("ready"),
            "target_state must be the NEW state the move drove to; got {event}"
        );
        assert_eq!(
            event["payload"]["actor"].as_str(),
            Some("pm"),
            "actor must be the role that performed the mutation; got {event}"
        );
        assert!(
            event["payload"]["ts"].as_str().is_some(),
            "the event carries a ts; got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}

#[test]
fn target_state_filter_discriminates_matching_from_non_matching_to_state() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mutator = TestBridge::connect(&bpmux, "pm").await;

        let story = create_target_story(
            &mutator,
            "[Feature] target-state-filter-target",
            "reject if a {target_state: ready} subscriber gets a non-ready transition",
        )
        .await;

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "story_state_transition",
                    "filter": { "target_state": "ready" }
                }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": story,
                    "target_state": "in-progress",
                    "rationale": "drive a NON-ready transition the ready-filter must reject"
                }),
            )
            .await
            .expect("move_story to in-progress succeeds");

        move_to_ready(&mutator, story).await;

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("wait_for_event delivers the ready transition, not a timeout");

        assert_eq!(
            event["payload"]["target_state"].as_str(),
            Some("ready"),
            "the FIRST received event must be the ready transition — discrimination proof under any load: the in-progress transition was driven FIRST, so were the target_state-filter to leak it the order-preserved dispatch would deliver in-progress FIRST and this assert would read 'in-progress' (no negative-timeout to outrun under CPU-starvation); got {event}"
        );
        assert_eq!(
            event["payload"]["story_id"].as_u64(),
            Some(story),
            "the delivered event must carry the moved story's id; got {event}"
        );
        assert_eq!(
            event["payload"]["from_state"].as_str(),
            Some("in-progress"),
            "from_state must reflect the prior (in-progress) state; got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}

#[test]
fn move_story_backward_rework_transition_still_fires_state_transition_event() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let agent = TestBridge::connect(&bpmux, "dev").await;
        let mutator = TestBridge::connect(&bpmux, "pm").await;

        let story = create_target_story(
            &mutator,
            "[Feature] rework-backward-emit-regression",
            "a backward (rework) move_story must still fire a story_state_transition event",
        )
        .await;

        mutator
            .call_tool(
                "move_story",
                serde_json::json!({
                    "number": story,
                    "target_state": "in-progress",
                    "rationale": "advance the story before driving the backward rework"
                }),
            )
            .await
            .expect("move_story to in-progress succeeds");

        let sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "story_state_transition",
                    "filter": { "story_id": story }
                }),
            )
            .await
            .expect("subscribe returns ok");
        let subscription_id = sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &subscription_id).await;

        move_to_ready(&mutator, story).await;

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 20000 }),
            )
            .await
            .expect("a backward rework move must fire a transition event, not a timeout");

        assert_eq!(
            event["payload"]["target_state"].as_str(),
            Some("ready"),
            "the backward in-progress→ready rework must report the AUTHORITATIVE target (ready) — regression for the dual-label drop where the post-dispatch decoder read resolved the transient ['ready','in progress'] labels to the more-advanced OLD state (in-progress), making target==from and SILENTLY dropping the transition; got {event}"
        );
        assert_eq!(
            event["payload"]["from_state"].as_str(),
            Some("in-progress"),
            "the rework's from_state must be the prior in-progress state; got {event}"
        );

        agent.close().await;
        mutator.close().await;
    });
}
