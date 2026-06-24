use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DoctrinePointer {
    pub path: String,
    pub last_updated_at: Option<String>,
    pub key_sections: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BriefingResponse {
    briefing: String,
    team_doctrine: Vec<DoctrinePointer>,
}

pub(in crate::bridge) async fn handle_get_briefing(
    args: &serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<String, String> {
    let default_role = name.split(':').next().unwrap_or(name);
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or(default_role);
    let briefing = crate::briefing::load_briefing(role);
    let docs_root = resolve_docs_root();
    let team_doctrine = build_team_doctrine_pointers(docs_root.as_deref());
    let body = BriefingResponse {
        briefing,
        team_doctrine,
    };
    serde_json::to_string(&body).map_err(|e| format!("get_briefing serialize: {e}"))
}

fn resolve_docs_root() -> Option<PathBuf> {
    if let Some(override_path) = std::env::var_os("BPMUX_DOCS_DIR") {
        return Some(PathBuf::from(override_path));
    }
    if let Ok(cwd) = std::env::current_dir() {
        for candidate in cwd.ancestors() {
            let probe = candidate.join("docs").join("process-state-machine.md");
            if probe.exists() {
                return Some(candidate.join("docs"));
            }
            let sibling = candidate.join("bpmux").join("docs");
            if sibling.join("process-state-machine.md").exists() {
                return Some(sibling);
            }
        }
    }
    None
}

const DOCTRINE_DOCS: &[(&str, &[&str])] = &[
    (
        "process-state-machine.md",
        &["States", "Gate Tools", "Cross-cutting Disciplines"],
    ),
    ("handoff-process.md", &["Roles", "Handoff Cycles", "Gates"]),
    (
        "memory-architecture.md",
        &["Storage Layers", "Promotion Criteria", "Identity Scope"],
    ),
];

pub fn build_team_doctrine_pointers(docs_root: Option<&Path>) -> Vec<DoctrinePointer> {
    let Some(root) = docs_root else {
        return DOCTRINE_DOCS
            .iter()
            .map(|(name, sections)| DoctrinePointer {
                path: format!("docs/{name}"),
                last_updated_at: None,
                key_sections: sections.iter().map(|s| s.to_string()).collect(),
            })
            .collect();
    };
    DOCTRINE_DOCS
        .iter()
        .map(|(name, sections)| {
            let full_path = root.join(name);
            let last_updated_at = read_mtime_rfc3339(&full_path);
            DoctrinePointer {
                path: format!("docs/{name}"),
                last_updated_at,
                key_sections: sections.iter().map(|s| s.to_string()).collect(),
            }
        })
        .collect()
}

fn read_mtime_rfc3339(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let datetime: chrono::DateTime<chrono::Utc> = modified.into();
    Some(datetime.to_rfc3339())
}
