#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use std::time::Duration;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use serde_json::Value;

async fn roster_has(bpmux: &TestBpmux, prefix: &str) -> bool {
    let url = format!("http://127.0.0.1:{}/api/roster", bpmux.port());
    let body: Value = reqwest::Client::new()
        .get(&url)
        .bearer_auth(bpmux.simulate_secret())
        .header(reqwest::header::HOST, format!("127.0.0.1:{}", bpmux.port()))
        .send()
        .await
        .expect("GET /api/roster")
        .json()
        .await
        .expect("parse roster json");
    body["roster"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|e| e["identity"].as_str())
                .any(|i| i.starts_with(prefix))
        })
        .unwrap_or(false)
}

async fn poll_until<F, Fut>(timeout: Duration, predicate: F) -> bool
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = bool>,
{
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if predicate().await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    false
}

#[test]
fn a_renamed_peer_receives_a_message_addressed_to_its_new_name() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut sender = WsPeer::connect(&bpmux, "pm").await;

        dev.rename("dev-renamed").await;
        let rename_landed = poll_until(Duration::from_secs(5), || async {
            roster_has(&bpmux, "dev-renamed").await
        })
        .await;
        assert!(
            rename_landed,
            "the rename must be processed server-side (reflected in the roster) before the cross-connection send"
        );

        sender.send_to("for the renamed dev", &["dev-renamed"]).await;

        let (from, text) = dev.recv_message(Duration::from_secs(5)).await;
        assert_eq!(
            text, "for the renamed dev",
            "the renamed peer must RECEIVE a message addressed to its NEW name — should_deliver matches the rename-updated live identity (the lived delivery-route, not the store-resolve cursor-key)"
        );
        assert_eq!(
            from.split(':').next().unwrap_or(&from),
            "pm",
            "the message is attributed to its real sender"
        );

        dev.close().await;
        sender.close().await;
    });
}

#[test]
fn after_a_rename_the_old_name_no_longer_routes_to_the_renamed_peer() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let mut sender = WsPeer::connect(&bpmux, "pm").await;

        dev.rename("dev-renamed").await;
        let rename_landed = poll_until(Duration::from_secs(5), || async {
            roster_has(&bpmux, "dev-renamed").await
        })
        .await;
        assert!(rename_landed, "rename must land before the send");

        sender.send_to("to the old name", &["dev"]).await;
        sender.send_to("to the new name", &["dev-renamed"]).await;

        let (_, first_received) = dev.recv_message(Duration::from_secs(5)).await;
        assert_eq!(
            first_received, "to the new name",
            "the renamed peer's FIRST received message is the one to its NEW name — the to-OLD message was sent FIRST but never arrives, so the old display is a true-dead route, not merely a slow delivery (catches a duplicate-route bug where rename copies instead of moves)"
        );

        dev.close().await;
        sender.close().await;
    });
}
