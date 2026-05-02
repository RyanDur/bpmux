//! #472: when a WS peer auto-identifies as `agent-XXXX` then renames
//! to a named role, the prior auto-identity must NOT remain in the
//! team-roster. Reproducer for the orphan-agent-XXXX-after-rename
//! sidebar Bug.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use std::time::Duration;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use serde_json::Value;

async fn roster_identities(bpmux: &TestBpmux) -> Vec<String> {
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
        .expect("roster array")
        .iter()
        .filter_map(|e| e["identity"].as_str().map(str::to_string))
        .collect()
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
fn rename_reaps_prior_auto_identity_from_roster() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "agent-abcd").await;

        let auto_present = poll_until(Duration::from_secs(5), || async {
            roster_identities(&bpmux)
                .await
                .iter()
                .any(|i| i.starts_with("agent-abcd"))
        })
        .await;
        assert!(
            auto_present,
            "auto-identity should appear in roster post-identify"
        );

        peer.rename("dev").await;

        let renamed = poll_until(Duration::from_secs(5), || async {
            let ids = roster_identities(&bpmux).await;
            ids.iter().any(|i| i.starts_with("dev"))
                && !ids.iter().any(|i| i.starts_with("agent-abcd"))
        })
        .await;
        let final_ids = roster_identities(&bpmux).await;
        assert!(
            renamed,
            "post-rename roster must contain dev but not agent-abcd; got {final_ids:?}"
        );

        peer.close().await;
    });
}
