#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{
    create_target_story, drive_to_delivered, seed_team_verifiable_evidence,
};
use std::time::Duration;

#[test]
fn accept_story_concurrent_fire_returns_target_state_invalid_on_second() {
    let bpmux = TestBpmux::start_with_envs(
        "human",
        &[
            ("BPMUX_TEST_RACE_WINDOW_REQUESTS", "2"),
            ("BPMUX_POLL_INTERVAL_SECS", "600"),
        ],
    );
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let pm2 = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let target = create_target_story(
            &pm,
            "[Feature] concurrent-accept-target",
            "reject if Sc 9 substrate-validator doesn't fire on the follower",
        )
        .await;

        drive_to_delivered(
            &pm,
            &dev,
            target,
            "applications/bpmux/tests/integration_main_modules/accept_story_concurrent_fire_e2e.rs",
        )
        .await;

        seed_team_verifiable_evidence(&bpmux, target).await;

        let (pm_outcome, pm2_outcome) = tokio::join!(
            pm.call_tool(
                "accept_story",
                serde_json::json!({"id": target, "would_file_bug": false}),
            ),
            pm2.call_tool(
                "accept_story",
                serde_json::json!({"id": target, "would_file_bug": false}),
            ),
        );

        let outcomes = [&pm_outcome, &pm2_outcome];
        let leader_count = outcomes.iter().filter(|r| r.is_ok()).count();
        let follower_count = outcomes.iter().filter(|r| r.is_err()).count();

        assert_eq!(
            leader_count, 1,
            "exactly one fire must win the race-to-mutate; got pm={pm_outcome:?} pm2={pm2_outcome:?}"
        );
        assert_eq!(
            follower_count, 1,
            "exactly one fire must see substrate-validator 409; got pm={pm_outcome:?} pm2={pm2_outcome:?}"
        );

        let follower_err = outcomes
            .iter()
            .find_map(|r| r.as_ref().err())
            .expect("one follower error");
        let follower_msg = follower_err.to_string();
        assert!(
            follower_msg.contains("workflow-target-state-invalid"),
            "follower must surface workflow-target-state-invalid; got {follower_msg}"
        );

        pm.close().await;
        dev.close().await;
        pm2.close().await;
    });
}
