//! Cross-handler helpers: workflow-label hygiene + role gating + a
//! story-existence ensure for canonical-store mutations.
//!
//! Workflow-label constants (`LABEL_DELIVERED` etc.) live here so
//! both the state-label set (`ALL_STATE_LABELS`) and the handler
//! call-sites can reference them without scattering string literals.
//! Extracted from `bridge::tools_stories` per #1353; consumers re-
//! reach these via `mod.rs::pub(crate) use helpers::{...}` re-exports
//! so existing import paths stay valid.

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

pub(crate) async fn ensure_story_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
) -> Result<crate::board::Story, String> {
    if let Some(story) = refresh.story(number) {
        return Ok(story);
    }
    refresh.refresh_story(number).await.map_err(|e| {
        format!(
            "story #{number} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown story state"
        )
    })?;
    refresh.story(number).ok_or_else(|| {
        format!(
            "story #{number} not found in canonical store after refresh_story \
             round-trip; story may not exist at source-of-truth"
        )
    })
}

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
    refresh: &dyn crate::board::RefreshDriver,
    number: u64,
    meta: &crate::board::events::EventMeta,
    keep: &str,
) -> Result<(), String> {
    let current = refresh
        .story(number)
        .map(|s| s.raw_labels)
        .unwrap_or_default();
    for stale in stale_workflow_labels(&current, keep) {
        refresh
            .mutate_sync(crate::board::BoardEvent::TagRemoved {
                meta: meta.clone(),
                number,
                label: stale.to_string(),
            })
            .map_err(|e| format!("#{number} {stale}-tag-remove enqueue failed: {e}"))?;
    }
    Ok(())
}

pub(crate) fn role_from_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

pub(crate) fn require_role(
    agent_name: &str,
    allowed: &[&str],
    operation: &str,
) -> Result<(), String> {
    let role = role_from_name(agent_name);
    if allowed.contains(&role) {
        Ok(())
    } else {
        Err(format!(
            "{operation} is restricted to {}. ask {} to {operation}.",
            allowed.join(" or "),
            allowed[0],
        ))
    }
}
