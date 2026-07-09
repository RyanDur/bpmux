//! #472: when a WS peer auto-identifies as `agent-XXXX` then renames
//! to a named role, the prior auto-identity must NOT remain in the
//! team-roster. Reproducer for the orphan-agent-XXXX-after-rename
//! sidebar Bug.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::harness;

use std::time::Duration;

use bpmux::transport::protocol::ServerMessage;
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
        let mut peer = WsPeer::connect_with_user_id(&bpmux, "agent-abcd", "sess-agent-abcd").await;

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

#[test]
fn rename_to_the_reserved_owner_identity_is_rejected_over_ws() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect_with_user_id(&bpmux, "dev", "sess-dev-reserved").await;

        peer.rename("owner").await;

        let rejected = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let Some(frame) = peer.next_text().await else {
                    return false;
                };
                if let Ok(ServerMessage::Error { reason }) =
                    serde_json::from_str::<ServerMessage>(&frame)
                {
                    return reason.to_lowercase().contains("owner");
                }
            }
        })
        .await
        .expect("a frame within 5s");
        assert!(
            rejected,
            "renaming to the OWNER identity over WS must be rejected with a ServerMessage::Error — the WS set_name-validation locus enforces the auto-submit attribution-boundary (an agent renamed to 'owner' would get owner-framed messages via frame_team_message's short_name(from)). This pins the WS wiring so a regression of the Rename arm cannot silently re-open owner-forgery CI-undetected, matching the REST register_rejects e2e"
        );

        let owner_not_assumed = poll_until(Duration::from_secs(3), || async {
            let ids = roster_identities(&bpmux).await;
            ids.iter().any(|i| i.starts_with("dev")) && !ids.iter().any(|i| i.starts_with("owner"))
        })
        .await;
        let final_ids = roster_identities(&bpmux).await;
        assert!(
            owner_not_assumed,
            "the rejected rename does NOT take effect — the peer stays dev, never becomes the owner; got {final_ids:?}"
        );

        peer.close().await;
    });
}
