use crate::product_management::story::StoryId;
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
    refresh: Option<&Arc<dyn crate::product_management::board::RefreshDriver>>,
    story_id: StoryId,
) -> Option<u64> {
    let story = refresh?.story(story_id.value())?;
    let class = crate::workflow::gate_class_from_labels_and_story_type(
        &story.raw_labels,
        story.story_type(),
    );
    let transition =
        crate::workflow::resolve_transition(crate::workflow::current_workflow(), "accept", class)
            .ok()?;
    Some(transition.evidence_window_seconds)
}

pub(in crate::bridge) async fn handle_view_verdicts(
    args: &serde_json::Map<String, serde_json::Value>,
    session: &Arc<crate::session::Session>,
    refresh: Option<&Arc<dyn crate::product_management::board::RefreshDriver>>,
) -> Result<String, String> {
    let story_id = StoryId::new(
        args.get("story_id")
            .and_then(|v| v.as_u64())
            .ok_or("missing 'story_id' argument")?,
    );

    let store = load(&session.dir);
    let records = verdicts_for_story(&store, story_id);
    let window_secs = resolve_accept_window_secs(refresh, story_id);
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
