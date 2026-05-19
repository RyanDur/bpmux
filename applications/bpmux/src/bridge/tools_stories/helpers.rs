//! Cross-handler helpers: workflow-label hygiene, role gating, body
//! composition, and acceptance-test-path validation.
//!
//! Workflow-label constants (`LABEL_DELIVERED` etc.) live here so
//! both the state-label set (`ALL_STATE_LABELS`) and the handler
//! call-sites can reference them without scattering string literals.
//! Body composers + path-validation helpers also live here as
//! cross-handler pure utilities. Extracted from `bridge::tools_stories`
//! per #1353; consumers reach these via `mod.rs` re-exports so
//! existing import paths stay valid.

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

/// Build the issue body for a new iteration-issue. Cassiopeia
/// substrate: iteration membership is canonical-via-GH via
/// `iteration-{Theme}.{position}` labels on member stories (#1300).
/// Body is human-readable, not parsed for membership.
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

/// Extract a trimmed-nonempty string from the JSON args map. Returns
/// the untrimmed raw slice so downstream formatting preserves
/// intentional leading/trailing whitespace in multi-line field values.
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

pub fn parse_acceptance_test_path(body: &str) -> Option<String> {
    let marker = "## Acceptance test path\n\n";
    let start = body.find(marker)? + marker.len();
    let rest = &body[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    let section = &rest[..end];
    for line in section.lines() {
        if let Some(path) = extract_path_from_line(line) {
            return Some(path);
        }
    }
    None
}

/// Tolerate the common prose shapes PM uses around the path: bullets
/// ("- Primary: tests/foo.rs"), labels ("Path: tests/foo.rs"),
/// parentheticals ("tests/foo.rs (per gate-3 contract)"). Returns the
/// first plausibly-path-shaped token on the line, or None when the
/// line carries no recognizable path.
fn extract_path_from_line(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let stripped = line
        .trim_start_matches(|c: char| c == '-' || c == '*' || c == '•' || c.is_whitespace())
        .trim();
    let after_label = match stripped.find(": ") {
        Some(idx) => &stripped[idx + 2..],
        None => stripped,
    };
    let trimmed = after_label
        .trim()
        .trim_matches(|c: char| c == '`' || c == '*' || c == '_');
    let token = match trimmed.find([' ', '\t', '(']) {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    let candidate = token
        .trim_end_matches(['.', ',', ';', '`', '*', '_'])
        .trim();
    if candidate.contains('/') && is_acceptable_acceptance_test_path(candidate) {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn is_acceptable_acceptance_test_path(candidate: &str) -> bool {
    const SOURCE_EXTS: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rb", ".java", ".kt", ".cs", ".cpp",
        ".c", ".h", ".hpp", ".swift", ".m",
    ];
    const OPS_EXTS: &[&str] = &[".md", ".yml", ".yaml", ".toml", ".json"];
    const OPS_PREFIXES: &[&str] = &["docs/", ".github/workflows/"];
    if SOURCE_EXTS.iter().any(|ext| candidate.ends_with(ext)) {
        return true;
    }
    OPS_PREFIXES
        .iter()
        .any(|prefix| candidate.starts_with(prefix))
        && OPS_EXTS.iter().any(|ext| candidate.ends_with(ext))
}

fn user_facing_story_type(t: crate::board::domain::StoryType) -> bool {
    matches!(
        t,
        crate::board::domain::StoryType::Feature | crate::board::domain::StoryType::Bug
    )
}

fn check_path_exists(path: &str) -> Result<(), String> {
    if let Some(resolved) = resolve_acceptance_test_path(path) {
        if resolved.exists() {
            return Ok(());
        }
        return Err(format!(
            "acceptance_test_path '{path}' resolved to {resolved:?} but file is missing. Create the test file before pickup OR fix the path. (repo-root: {:?}; cwd: {:?})",
            find_repo_root(),
            std::env::current_dir().ok(),
        ));
    }
    Err(format!(
        "acceptance_test_path '{path}' does not exist; tried absolute, repo-root-relative, and cwd-walk shapes. Create the test file before pickup OR fix the path. (repo-root: {:?}; cwd: {:?})",
        find_repo_root(),
        std::env::current_dir().ok(),
    ))
}

pub(crate) fn resolve_acceptance_test_path(path: &str) -> Option<std::path::PathBuf> {
    let raw = std::path::PathBuf::from(path);
    if raw.is_absolute() {
        return raw.exists().then_some(raw);
    }
    if let Some(root) = find_repo_root() {
        let from_root = root.join(path);
        if from_root.exists() {
            return Some(from_root);
        }
    }
    let cwd_candidates = [
        raw.clone(),
        std::path::PathBuf::from("..").join(path),
        std::path::PathBuf::from("../..").join(path),
    ];
    cwd_candidates.into_iter().find(|p| p.exists())
}

pub(crate) fn find_repo_root() -> Option<std::path::PathBuf> {
    if let Some(start) = std::env::current_dir().ok()
        && let Some(root) = walk_for_git_root(&start)
    {
        return Some(root);
    }
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    walk_for_git_root(&manifest_dir)
}

fn walk_for_git_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

pub(crate) fn validate_acceptance_test_path_at_pickup(
    args: &serde_json::Map<String, serde_json::Value>,
    number: u64,
    refresh: &dyn crate::board::RefreshDriver,
) -> Result<(), String> {
    let Some(story) = refresh.story(number) else {
        return Ok(());
    };
    if !user_facing_story_type(story.story_type) {
        return Ok(());
    }
    let arg_path = args
        .get("acceptance_test_path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(path) = arg_path.or(story.acceptance_test_path.as_deref()) else {
        return Ok(());
    };
    check_path_exists(path)
}

pub(crate) fn validate_acceptance_test_path_at_deliver(
    number: u64,
    refresh: &dyn crate::board::RefreshDriver,
) -> Result<(), String> {
    let Some(story) = refresh.story(number) else {
        return Ok(());
    };
    if !user_facing_story_type(story.story_type) {
        return Ok(());
    }
    let Some(path) = story.acceptance_test_path.as_deref() else {
        return Ok(());
    };
    check_path_exists(path)
}
