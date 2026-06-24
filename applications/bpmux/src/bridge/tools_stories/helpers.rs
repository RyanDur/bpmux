pub(crate) const LABEL_DELIVERED: &str = "delivered";
pub(crate) const LABEL_IN_PROGRESS: &str = "in progress";
pub(crate) const LABEL_READY: &str = "ready";
pub(crate) const LABEL_READY_FOR_ACCEPT: &str = "ready for accept";

pub(crate) const LABEL_AWAITING_TEAM: &str = "awaiting team";
pub(crate) const LABEL_AWAITING_OWNER: &str = "awaiting owner";
pub(crate) const LABEL_AWAITING_DEV: &str = "awaiting dev";

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
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
) -> Result<crate::product_management::board::Story, String> {
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
            .mutate_sync(crate::product_management::board::BoardEvent::TagRemoved {
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
    if let Some(owner) = crate::workflow::current_workflow().owner_role_id()
        && !owner.is_empty()
        && role == owner
    {
        return Ok(());
    }
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

pub(crate) fn require_purpose(
    agent_name: &str,
    purposes: &[&str],
    operation: &str,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let allowed: Vec<String> = purposes
        .iter()
        .map(|p| {
            workflow
                .role_id_for_purpose(p)
                .map(str::to_string)
                .unwrap_or_else(|| (*p).to_string())
        })
        .collect();
    let allowed_refs: Vec<&str> = allowed.iter().map(String::as_str).collect();
    require_role(agent_name, &allowed_refs, operation)
}

pub(crate) fn require_purpose_any(
    agent_name: &str,
    purpose: &str,
    operation: &str,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let allowed = workflow.role_ids_for_purpose(purpose);
    if allowed.is_empty() {
        return Err(format!("{operation} has no configured {purpose} role"));
    }
    require_role(agent_name, &allowed, operation)
}

pub(crate) fn compose_iteration_body(theme_name: &str) -> String {
    let today = chrono::Utc::now().format("%Y-%m-%d");
    format!(
        "## Iteration {theme_name}\n\
         \n\
         **Created**: {today}\n\
         **Status**: Open\n\
         \n\
         ## Member stories\n\
         \n\
         <!-- Members carry `iteration-{theme_name}.{{position}}` labels;\n\
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
