use std::future::Future;

use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

pub(in crate::bridge) const VIEW_VELOCITY_MARKER: &str = "__BPMUX_VIEW_VELOCITY__";

pub(in crate::bridge) async fn fetch_messages_via_http(
    port: u16,
) -> Result<Vec<crate::message::Message>, String> {
    let url = crate::transport::local_url::api_url(port, "/api/messages?limit=2000");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    serde_json::from_str(&body).map_err(|e| format!("parse messages: {e}: {body}"))
}

pub(in crate::bridge) async fn handle_view_velocity<F, Fut>(
    fetch_messages: F,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Vec<crate::message::Message>, String>>,
{
    let messages = fetch_messages().await?;

    let mut unique_numbers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for m in &messages {
        if let Some(hash_idx) = m.text.find('#')
            && let Some((num_str, _)) = m.text[hash_idx + 1..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|i| m.text[hash_idx + 1..].split_at(i))
            && let Ok(n) = num_str.parse::<u64>()
        {
            unique_numbers.insert(n);
        }
    }
    let mut feature_numbers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for n in unique_numbers {
        if is_feature_for_velocity(n, refresh) {
            feature_numbers.insert(n);
        }
    }

    let metrics = crate::velocity::compute(&messages, chrono::Utc::now(), |n| {
        feature_numbers.contains(&n)
    });

    let marker_msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_VELOCITY_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
    };
    match serde_json::to_string(&marker_msg) {
        Ok(json) => {
            if let Err(e) = ws_send(ws_tx, json).await {
                crate::diag::log_diag(&format!("view_velocity marker send failed: {e}"));
            }
        }
        Err(e) => crate::diag::log_diag(&format!("view_velocity marker serialize failed: {e}")),
    }

    let view = crate::bridge::view_model::VelocityMetricsView::from(&metrics);
    serde_json::to_string_pretty(&view).map_err(|e| format!("serialize: {e}"))
}

pub(in crate::bridge) fn is_feature_for_velocity(
    n: u64,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> bool {
    refresh
        .and_then(|r| r.story(n))
        .is_some_and(|s| matches!(s.story_type, crate::board::domain::StoryType::Feature))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn orchestrator_with_seed(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&store),
            Arc::clone(&adapter),
        ));
        orch.refresh_all().await.unwrap();
        orch
    }

    fn make_story(number: u64, story_type: StoryType) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("story-{number}"),
            StoryState::InProgress,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        s.story_type = story_type;
        s
    }

    #[tokio::test]
    async fn refreshed_path_classifies_feature_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(901, StoryType::Feature)]).await;
        assert!(is_feature_for_velocity(901, Some(&orch)));
    }

    #[tokio::test]
    async fn refreshed_path_rejects_bug_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(902, StoryType::Bug)]).await;
        assert!(!is_feature_for_velocity(902, Some(&orch)));
    }

    #[tokio::test]
    async fn refreshed_path_rejects_chore_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(903, StoryType::Chore)]).await;
        assert!(!is_feature_for_velocity(903, Some(&orch)));
    }

    #[tokio::test]
    async fn returns_false_when_refresh_absent() {
        assert!(!is_feature_for_velocity(904, None));
    }

    #[tokio::test]
    async fn returns_false_when_story_not_in_cache() {
        let orch = orchestrator_with_seed(vec![]).await;
        assert!(!is_feature_for_velocity(905, Some(&orch)));
    }
}
