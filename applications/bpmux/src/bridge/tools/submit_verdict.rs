use crate::session::verdicts::{AddVerdictOutcome, Verdict, add_or_replace_verdict, load, save};
use std::sync::Arc;

pub(in crate::bridge) async fn handle_submit_verdict(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    session: &Arc<crate::session::Session>,
) -> Result<String, String> {
    let role = agent_name.split(':').next().unwrap_or(agent_name);
    if !matches!(role, "dev" | "designer") {
        return Err(format!(
            "submit_verdict is restricted to dev or designer; agent '{agent_name}' is role '{role}'"
        ));
    }
    let story_number = args
        .get("story_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'story_number' argument")?;
    let verdict_value = args
        .get("verdict")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("missing 'verdict' argument (non-empty string)")?
        .to_string();
    let substrate_grounding = args
        .get("substrate_grounding")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or("missing 'substrate_grounding' argument (non-empty string)")?
        .to_string();

    let entry = Verdict {
        story_number,
        from_role: role.to_string(),
        verdict: verdict_value,
        substrate_grounding,
        ts: chrono::Utc::now(),
    };

    let mut store = load(&session.dir);
    let outcome = add_or_replace_verdict(&mut store, entry);
    save(&session.dir, &store).map_err(|e| format!("save verdicts: {e}"))?;

    let outcome_label = match outcome {
        AddVerdictOutcome::Inserted => "new",
        AddVerdictOutcome::Replaced => "replaced prior",
    };
    Ok(format!(
        "verdict recorded on #{story_number} from {role} ({outcome_label})"
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::session::log::Session;
    use crate::session::verdicts::{load, verdicts_for_story};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use tokio::sync::broadcast;

    fn make_session(dir: PathBuf) -> Arc<Session> {
        let (tx, _) = broadcast::channel(16);
        Arc::new(Session::new(dir, "ryan".to_string(), tx))
    }

    fn make_args(
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

    #[tokio::test]
    async fn dev_writes_verdict_to_store_at_session_dir() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let args = make_args(42, "verified", "src/foo.rs:10 cite");
        let result = handle_submit_verdict(&args, "dev:claude-code", &session)
            .await
            .expect("dev fire should succeed");
        assert!(result.contains("verdict recorded on #42 from dev"));
        let store = load(&session.dir);
        let stored = verdicts_for_story(&store, 42);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].from_role, "dev");
        assert_eq!(stored[0].verdict, "verified");
        assert_eq!(stored[0].substrate_grounding, "src/foo.rs:10 cite");
    }

    #[tokio::test]
    async fn designer_writes_verdict_to_store() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let args = make_args(99, "rejected", "missing acceptance cite");
        handle_submit_verdict(&args, "designer:claude-code", &session)
            .await
            .expect("designer fire should succeed");
        let store = load(&session.dir);
        let stored = verdicts_for_story(&store, 99);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].from_role, "designer");
    }

    #[tokio::test]
    async fn pm_role_is_refused() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let args = make_args(42, "verified", "cite");
        let err = handle_submit_verdict(&args, "pm:claude-code", &session)
            .await
            .expect_err("pm fire should refuse");
        assert!(err.contains("restricted to dev or designer"));
        assert!(err.contains("pm"));
    }

    #[tokio::test]
    async fn missing_substrate_grounding_is_refused() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let mut args = serde_json::Map::new();
        args.insert("story_number".into(), serde_json::json!(42));
        args.insert("verdict".into(), serde_json::json!("verified"));
        let err = handle_submit_verdict(&args, "dev", &session)
            .await
            .expect_err("missing substrate_grounding should refuse");
        assert!(err.contains("substrate_grounding"));
    }

    #[tokio::test]
    async fn empty_substrate_grounding_is_refused() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let args = make_args(42, "verified", "   ");
        let err = handle_submit_verdict(&args, "dev", &session)
            .await
            .expect_err("whitespace-only substrate_grounding should refuse");
        assert!(err.contains("substrate_grounding"));
    }

    #[tokio::test]
    async fn replay_from_same_role_replaces_prior() {
        let dir = TempDir::new().unwrap();
        let session = make_session(dir.path().to_path_buf());
        let first = make_args(42, "verified", "initial cite");
        handle_submit_verdict(&first, "dev", &session)
            .await
            .unwrap();
        let second = make_args(42, "rejected", "updated cite after second pass");
        let result = handle_submit_verdict(&second, "dev", &session)
            .await
            .expect("replay should succeed");
        assert!(result.contains("replaced prior"));
        let store = load(&session.dir);
        let stored = verdicts_for_story(&store, 42);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].verdict, "rejected");
        assert_eq!(
            stored[0].substrate_grounding,
            "updated cite after second pass"
        );
    }
}
