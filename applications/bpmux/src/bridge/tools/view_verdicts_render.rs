#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view_verdicts::*;
use crate::bridge::tools::handle_submit_verdict;
use crate::hub::Hub;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;

fn make_hub(dir: PathBuf) -> Arc<Hub> {
    let (tx, _) = broadcast::channel(16);
    Arc::new(Hub::new(dir, "owner".to_string(), tx))
}

fn submit_args(
    id: u64,
    verdict: &str,
    substrate_grounding: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("story_id".into(), serde_json::json!(id));
    m.insert("verdict".into(), serde_json::json!(verdict));
    m.insert(
        "substrate_grounding".into(),
        serde_json::json!(substrate_grounding),
    );
    m
}

fn view_args(id: u64) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert("story_id".into(), serde_json::json!(id));
    m
}

#[tokio::test]
async fn returns_record_after_write() {
    let dir = TempDir::new().unwrap();
    let hub = make_hub(dir.path().to_path_buf());
    handle_submit_verdict(
        &submit_args(42, "verified", "src/foo.rs cite"),
        "designer:claude-code",
        &hub,
    )
    .await
    .unwrap();
    let response = handle_view_verdicts(&view_args(42), &hub, None)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
    let records = parsed.as_array().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["from_role"], "designer");
    assert_eq!(records[0]["verdict"], "verified");
    assert_eq!(records[0]["substrate_grounding"], "src/foo.rs cite");
}
