#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::submit_verdict::*;
use crate::hub::Hub;
use crate::product_management::evidence::{evidence_for_story, load};
use crate::product_management::story::StoryId;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;

fn make_hub(dir: PathBuf) -> Arc<Hub> {
    let (tx, _) = broadcast::channel(16);
    Arc::new(Hub::new(dir, "owner".to_string(), tx))
}

fn make_args(
    number: u64,
    verdict: &str,
    substrate_grounding: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("story_id".into(), serde_json::json!(number));
    m.insert("verdict".into(), serde_json::json!(verdict));
    m.insert(
        "substrate_grounding".into(),
        serde_json::json!(substrate_grounding),
    );
    m
}

fn make_hub_with_rx(
    dir: PathBuf,
) -> (
    Arc<Hub>,
    tokio::sync::broadcast::Receiver<crate::message::Message>,
) {
    let (tx, rx) = broadcast::channel(16);
    let hub = Arc::new(Hub::new(dir, "owner".to_string(), tx));
    (hub, rx)
}

#[tokio::test]
async fn verdict_cast_broadcast_carries_message_type_system_not_chat() {
    let dir = TempDir::new().unwrap();
    let (hub, mut rx) = make_hub_with_rx(dir.path().to_path_buf());
    let args = make_args(
        1494,
        "verified",
        "src/bridge/tools/submit_verdict.rs:54 cite",
    );
    handle_submit_verdict(&args, "dev:claude-code", &hub)
        .await
        .expect("dev fire should succeed");
    let received = rx.try_recv().expect("broadcast should arrive");
    assert_eq!(received.from, "bpmux");
    assert_eq!(
        received.message_type,
        crate::message::MessageType::System,
        "verdict-cast notification must use MessageType::System so chat-filter bpmux pill catches it; default Chat lets the message leak past the consumer filter (sister to #1456 producer-axis bug)"
    );
    let wire = serde_json::to_value(&received).expect("serialize");
    assert_eq!(
        wire.get("message_type").and_then(|v| v.as_str()),
        Some("system"),
        "wire payload must serialize message_type=\"system\" so renderer classOf-remap classifies as bpmux-class"
    );
}

#[tokio::test]
async fn dev_writes_verdict_to_store_at_hub_dir() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let args = make_args(42, "verified", "src/foo.rs:10 cite");
    let result = handle_submit_verdict(&args, "dev:claude-code", &hub)
        .await
        .expect("dev fire should succeed");
    assert!(result.contains("verdict recorded on #42 from dev"));
    let store = load(&hub.dir);
    let stored = evidence_for_story(&store, StoryId::new(42));
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].from_role, "dev");
    assert_eq!(stored[0].verdict, "verified");
    assert_eq!(stored[0].substrate_grounding, "src/foo.rs:10 cite");
}

#[tokio::test]
async fn designer_writes_verdict_to_store() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let args = make_args(99, "rejected", "missing acceptance cite");
    handle_submit_verdict(&args, "designer:claude-code", &hub)
        .await
        .expect("designer fire should succeed");
    let store = load(&hub.dir);
    let stored = evidence_for_story(&store, StoryId::new(99));
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].from_role, "designer");
}

#[tokio::test]
async fn pm_role_is_refused() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let args = make_args(42, "verified", "cite");
    let err = handle_submit_verdict(&args, "pm:claude-code", &hub)
        .await
        .expect_err("pm fire should refuse");
    assert!(err.contains("restricted to dev or designer"));
    assert!(err.contains("pm"));
}

#[tokio::test]
async fn missing_substrate_grounding_is_refused() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let mut args = serde_json::Map::new();
    args.insert("story_id".into(), serde_json::json!(42));
    args.insert("verdict".into(), serde_json::json!("verified"));
    let err = handle_submit_verdict(&args, "dev", &hub)
        .await
        .expect_err("missing substrate_grounding should refuse");
    assert!(err.contains("substrate_grounding"));
}

#[tokio::test]
async fn empty_substrate_grounding_is_refused() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let args = make_args(42, "verified", "   ");
    let err = handle_submit_verdict(&args, "dev", &hub)
        .await
        .expect_err("whitespace-only substrate_grounding should refuse");
    assert!(err.contains("substrate_grounding"));
}

#[tokio::test]
async fn replay_from_same_role_replaces_prior() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    let first = make_args(42, "verified", "initial cite");
    handle_submit_verdict(&first, "dev", &hub).await.unwrap();
    let second = make_args(42, "rejected", "updated cite after second pass");
    let result = handle_submit_verdict(&second, "dev", &hub)
        .await
        .expect("replay should succeed");
    assert!(result.contains("replaced prior"));
    let store = load(&hub.dir);
    let stored = evidence_for_story(&store, StoryId::new(42));
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].verdict, "rejected");
    assert_eq!(
        stored[0].substrate_grounding,
        "updated cite after second pass"
    );
}
