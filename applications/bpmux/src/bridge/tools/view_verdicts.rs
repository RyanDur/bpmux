use crate::workflow::verdicts::{Verdict, load, verdicts_for_story};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct VerdictView {
    #[serde(flatten)]
    verdict: Verdict,
    #[serde(skip_serializing_if = "Option::is_none")]
    fresh: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    substrate_changed: Option<bool>,
}

fn resolve_accept_window_secs(
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
    story_number: u64,
) -> Option<u64> {
    let story = refresh?.store().story(story_number)?;
    let class =
        crate::workflow::gate_class_from_labels_and_story_type(&story.raw_labels, story.story_type);
    let transition =
        crate::workflow::resolve_transition(crate::workflow::current_workflow(), "accept", class)
            .ok()?;
    Some(transition.evidence_window_seconds)
}

pub(in crate::bridge) async fn handle_view_verdicts(
    args: &serde_json::Map<String, serde_json::Value>,
    session: &Arc<crate::session::Session>,
    refresh: Option<&Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let story_number = args
        .get("story_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_number' argument")?;

    let store = load(&session.dir);
    let records = verdicts_for_story(&store, story_number);
    let window_secs = resolve_accept_window_secs(refresh, story_number);
    let current_head = crate::bridge::tools::git_repo::current_head_sha();
    let now = chrono::Utc::now();
    let views: Vec<VerdictView> = records
        .into_iter()
        .map(|verdict| {
            let state = window_secs.map(|w| {
                crate::workflow::verdict_freshness(
                    Some(verdict.ts),
                    w,
                    now,
                    verdict.grounded_head.as_deref(),
                    current_head.as_deref(),
                )
            });
            VerdictView {
                fresh: state.as_ref().map(|s| s.is_fresh()),
                expires_at: state.as_ref().and_then(|s| s.expired_at),
                substrate_changed: state.as_ref().map(|s| s.substrate_changed),
                verdict,
            }
        })
        .collect();
    serde_json::to_string(&views).map_err(|e| format!("serialize verdicts: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::bridge::tools::handle_submit_verdict;
    use crate::session::Session;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    fn make_session(dir: PathBuf) -> Arc<Session> {
        let (tx, _) = broadcast::channel(16);
        Arc::new(Session::new(dir, "owner".to_string(), tx))
    }

    fn submit_args(
        number: u64,
        verdict: &str,
        substrate_grounding: &str,
    ) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("story_number".into(), serde_json::json!(number));
        m.insert("verdict".into(), serde_json::json!(verdict));
        m.insert(
            "substrate_grounding".into(),
            serde_json::json!(substrate_grounding),
        );
        m
    }

    fn view_args(number: u64) -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("story_number".into(), serde_json::json!(number));
        m
    }

    #[tokio::test]
    async fn returns_record_after_write() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        handle_submit_verdict(
            &submit_args(42, "verified", "src/foo.rs cite"),
            "designer:claude-code",
            &session,
        )
        .await
        .unwrap();
        let response = handle_view_verdicts(&view_args(42), &session, None)
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&response).unwrap();
        let records = parsed.as_array().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["from_role"], "designer");
        assert_eq!(records[0]["verdict"], "verified");
        assert_eq!(records[0]["substrate_grounding"], "src/foo.rs cite");
    }
}
