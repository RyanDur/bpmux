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

        let story = create_target_story(
            &mutator,
            "[Feature] story-transition-id-target",
            "reject if a transition on a NON-filtered story_id leaks through",
        )
        .await;
        let other_id = story + 9999;

        let wrong_sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "story_state_transition",
                    "filter": { "story_id": other_id }
                }),
            )
            .await
            .expect("subscribe (wrong story_id) returns ok");
        let wrong_subscription_id = wrong_sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        let right_sub = agent
            .call_tool(
                "subscribe",
                serde_json::json!({
                    "event_class": "story_state_transition",
                    "filter": { "story_id": story }
                }),
            )
            .await
            .expect("subscribe (matching story_id) returns ok");
        let right_subscription_id = right_sub["subscription_id"]
            .as_str()
            .expect("subscribe returns a subscription_id")
            .to_string();

        drain_pending_events(&agent, &wrong_subscription_id).await;
        drain_pending_events(&agent, &right_subscription_id).await;

        move_to_ready(&mutator, story).await;

        let leaked: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": wrong_subscription_id, "timeout_ms": 1500 }),
            )
            .await
            .expect("wait_for_event on the wrong-id subscription");
        assert_eq!(
            leaked["timed_out"].as_bool(),
            Some(true),
            "the real story's transition must NOT reach a subscriber filtered to a DIFFERENT story_id — non-vacuity: if the filter were absent, this event would leak here; got {leaked}"
        );

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": right_subscription_id, "timeout_ms": 5000 }),
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
            "the delivered event must carry the moved story's id; got {event}"
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

        let leaked: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 1500 }),
            )
            .await
            .expect("wait_for_event after the NON-matching (in-progress) transition");
        assert_eq!(
            leaked["timed_out"].as_bool(),
            Some(true),
            "an in-progress transition must NOT reach a {{target_state: ready}}-filtered subscriber — non-vacuity: absent the filter, this would leak; got {leaked}"
        );

        move_to_ready(&mutator, story).await;

        let event: Value = agent
            .call_tool(
                "wait_for_event",
                serde_json::json!({ "subscription_id": subscription_id, "timeout_ms": 5000 }),
            )
            .await
            .expect("wait_for_event delivers the ready transition, not a timeout");

        assert_eq!(
            event["payload"]["target_state"].as_str(),
            Some("ready"),
            "the {{target_state: ready}} filter must deliver exactly the ready transition; got {event}"
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
