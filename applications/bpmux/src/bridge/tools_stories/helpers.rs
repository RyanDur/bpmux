pub(crate) use super::authorization::{require_purpose, require_purpose_any, role_from_name};
pub(crate) use super::canonical_store::{
    ensure_epic_in_canonical_store, ensure_iteration_in_canonical_store,
    ensure_outcome_in_canonical_store, ensure_story_in_canonical_store,
};
use crate::product_management::story::StoryId;

pub(crate) const LABEL_DELIVERED: &str = "delivered";
pub(crate) const LABEL_IN_PROGRESS: &str = "in progress";
pub(crate) const LABEL_READY: &str = "ready";
pub(crate) const LABEL_READY_FOR_ACCEPT: &str = "ready for accept";

pub(crate) const ALL_STATE_LABELS: &[&str] = &[
    "icebox",
    LABEL_READY,
    LABEL_IN_PROGRESS,
    "desk-check",
    LABEL_DELIVERED,
    LABEL_READY_FOR_ACCEPT,
    "needs fixes",
    "rejected",
    "accepted",
];

pub(crate) const KEEP_NONE: &str = "";

pub(crate) fn stale_workflow_labels<'a>(current: &'a [String], keep: &str) -> Vec<&'a str> {
    current
        .iter()
        .filter_map(|l| {
            let l_str = l.as_str();
            if l_str == keep {
                return None;
            }
            ALL_STATE_LABELS
                .iter()
                .find(|s| **s == l_str)
                .map(|_| l_str)
        })
        .collect()
}

pub(crate) fn reap_workflow_labels(
    refresh: &dyn crate::product_management::board::RefreshDriver,
    number: u64,
    meta: &crate::product_management::board::events::EventMeta,
    keep: &str,
) -> Result<(), String> {
    let current = refresh
        .story(number)
        .map(|s| s.raw_labels)
        .unwrap_or_default();
    for stale in stale_workflow_labels(&current, keep) {
        refresh
            .mutate_sync(crate::product_management::board::StoryEvent::TagRemoved {
                meta: meta.clone(),
                id: StoryId::new(number),
                label: stale.to_string(),
            })
            .map_err(|e| format!("#{number} {stale}-tag-remove enqueue failed: {e}"))?;
    }
    Ok(())
}

pub(crate) fn compose_iteration_body(name: &str) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d");
    format!(
        "## Iteration {name}\n\
         \n\
         **Created**: {today}\n\
         **Status**: Open\n\
         \n\
         ## Member stories\n\
         \n\
         <!-- Members carry `iteration-{name}.{{position}}` labels;\n\
              ordering is by position ascending. -->\n\
         \n\
         _(no members yet)_\n\
         \n\
         ## Notes\n\
         \n\
         bpmux iteration substrate (Epic #1189 architectural-pivot).\n\
         Close via `close_iteration` ceremony.\n"
    )
}

pub(crate) fn require_nonempty_string<'a>(
    args: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    error_doc: &str,
) -> Result<&'a str, String> {
    let val = args.get(key).and_then(|v| v.as_str()).unwrap_or("").trim();
    if val.is_empty() {
        return Err(error_doc.to_string());
    }
    let raw = args.get(key).and_then(|v| v.as_str()).unwrap_or_default();
    Ok(raw)
}

pub(crate) fn compose_story_body(
    user_job: &str,
    observation_falsifier: &str,
    acceptance_test_path: Option<&str>,
    author_body: &str,
) -> String {
    let mut out = String::new();
    out.push_str("## User job\n\n");
    out.push_str(user_job.trim());
    out.push_str("\n\n## Observation falsifier\n\n");
    out.push_str(observation_falsifier.trim());
    if let Some(path) = acceptance_test_path {
        out.push_str("\n\n## Acceptance test path\n\n");
        out.push_str(path);
    }
    out.push_str("\n\n---\n\n");
    out.push_str(author_body);
    out
}
