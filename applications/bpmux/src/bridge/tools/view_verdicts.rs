use crate::session::verdicts::{load, verdicts_for_story};
use std::sync::Arc;

pub(in crate::bridge) async fn handle_view_verdicts(
    args: &serde_json::Map<String, serde_json::Value>,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let story_number = args
        .get("story_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_number' argument")?;

    let store = load(&session.dir);
    let records = verdicts_for_story(&store, story_number);
    serde_json::to_string(&records).map_err(|e| format!("serialize verdicts: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::bridge::tools::handle_submit_verdict;
    use crate::session::log::Session;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    fn make_session(dir: PathBuf) -> Arc<Session> {
        let (tx, _) = broadcast::channel(16);
        Arc::new(Session::new(dir, "ryan".to_string(), tx))
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
        let response = handle_view_verdicts(&view_args(42), &session)
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
