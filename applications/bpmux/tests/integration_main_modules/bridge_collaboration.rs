//! Agent-to-agent collaboration test: unmentioned messages still notify.
//!
//! Proves: when agent A sends a message that does not @mention agent B,
//! agent B still receives a channel notification for it. This removes the
//! parallel-monologue failure mode where agents would reply to a human
//! simultaneously and never see each other's replies.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn unmentioned_agent_messages_notify_other_agents() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;

        dev.send("starting the implementation").await;

        pm.capture
            .wait_for("starting the implementation", Duration::from_secs(5))
            .await;

        dev.close().await;
        pm.close().await;
    });
}
