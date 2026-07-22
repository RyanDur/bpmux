pub use super::authorization::{require_purpose, require_purpose_any, role_from_name};
pub use super::canonical_store::{
    ensure_epic_in_canonical_store, ensure_iteration_in_canonical_store,
    ensure_outcome_in_canonical_store, ensure_story_in_canonical_store,
};
pub fn compose_iteration_body(name: &str) -> String {
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

pub fn require_nonempty_string<'a>(
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

pub fn compose_story_body(
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
