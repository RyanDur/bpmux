//! Read-side of the post-compact briefing pair (#287). Returns the
//! caller's own briefing by default; pass `role` to read another
//! role's (peer-read is allowed per the individuality-plus-emergent-
//! team discipline — transparency without merging).
//!
//! #1148: response now includes a `team_doctrine` pointer-list with
//! filesystem-mtime so process-doctrine reaches all agents
//! structurally without baking content into per-role CLAUDE.md.
//! Pointer-shape (path + last_updated_at + key_sections) keeps
//! per-call context-budget under ~2KB; agents read full content
//! on-demand via the Read tool when relevant.

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
    let briefing = crate::session::briefing::load_briefing(role);
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn write_doc(dir: &Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).expect("write doc");
    }

    #[test]
    fn build_pointers_returns_three_canonical_doctrine_docs() {
        let pointers = build_team_doctrine_pointers(None);
        assert_eq!(
            pointers.len(),
            3,
            "must surface all three canonical doctrine docs"
        );
        let paths: Vec<&str> = pointers.iter().map(|p| p.path.as_str()).collect();
        assert!(paths.contains(&"docs/process-state-machine.md"));
        assert!(paths.contains(&"docs/handoff-process.md"));
        assert!(paths.contains(&"docs/memory-architecture.md"));
    }

    #[test]
    fn build_pointers_returns_none_mtime_when_docs_root_is_none() {
        let pointers = build_team_doctrine_pointers(None);
        assert!(
            pointers.iter().all(|p| p.last_updated_at.is_none()),
            "no docs-root → no mtime; pointers still surfaced for context"
        );
    }

    #[test]
    fn build_pointers_extracts_mtime_when_docs_present() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        write_doc(tmp.path(), "handoff-process.md", "# Roles");
        write_doc(tmp.path(), "memory-architecture.md", "# Storage Layers");
        let pointers = build_team_doctrine_pointers(Some(tmp.path()));
        assert!(
            pointers.iter().all(|p| p.last_updated_at.is_some()),
            "all three docs must have mtime when present on disk"
        );
    }

    #[test]
    fn build_pointers_missing_doc_yields_none_mtime_for_that_doc_only() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        let pointers = build_team_doctrine_pointers(Some(tmp.path()));
        let by_path: std::collections::HashMap<&str, &DoctrinePointer> =
            pointers.iter().map(|p| (p.path.as_str(), p)).collect();
        assert!(
            by_path["docs/process-state-machine.md"]
                .last_updated_at
                .is_some(),
            "present doc has mtime"
        );
        assert!(
            by_path["docs/handoff-process.md"].last_updated_at.is_none(),
            "missing doc has no mtime; pointer still surfaced"
        );
    }

    #[test]
    fn build_pointers_total_size_under_2kb_per_context_budget_discipline() {
        let pointers = build_team_doctrine_pointers(None);
        let serialized = serde_json::to_string(&pointers).unwrap();
        assert!(
            serialized.len() < 2048,
            "team_doctrine serialized payload must stay under 2KB; got {} bytes",
            serialized.len()
        );
    }

    #[test]
    fn build_pointers_each_pointer_has_at_least_one_key_section() {
        let pointers = build_team_doctrine_pointers(None);
        for p in &pointers {
            assert!(
                !p.key_sections.is_empty(),
                "{} must surface at least one key section",
                p.path
            );
        }
    }

    #[test]
    fn build_pointers_mtime_is_rfc3339_with_tz_designator() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        let pointers = build_team_doctrine_pointers(Some(tmp.path()));
        let p = pointers
            .iter()
            .find(|p| p.path == "docs/process-state-machine.md")
            .unwrap();
        let mtime = p.last_updated_at.as_ref().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(mtime).is_ok(),
            "mtime must be RFC3339-parseable; got {mtime}"
        );
    }

    #[test]
    fn build_pointers_freshness_within_1min_of_doc_edit() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        let now = SystemTime::now();
        let pointers = build_team_doctrine_pointers(Some(tmp.path()));
        let p = pointers
            .iter()
            .find(|p| p.path == "docs/process-state-machine.md")
            .unwrap();
        let mtime: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::parse_from_rfc3339(p.last_updated_at.as_ref().unwrap())
                .unwrap()
                .with_timezone(&chrono::Utc);
        let now_dt: chrono::DateTime<chrono::Utc> = now.into();
        let delta = (now_dt - mtime).num_seconds().abs();
        assert!(
            delta <= 60,
            "mtime must be within 1min of doc-edit per falsifier; got delta {delta}s"
        );
    }

    #[test]
    fn build_pointers_stable_when_doc_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        let p1 = build_team_doctrine_pointers(Some(tmp.path()));
        std::thread::sleep(Duration::from_millis(50));
        let p2 = build_team_doctrine_pointers(Some(tmp.path()));
        assert_eq!(
            p1.iter()
                .find(|p| p.path == "docs/process-state-machine.md")
                .unwrap()
                .last_updated_at,
            p2.iter()
                .find(|p| p.path == "docs/process-state-machine.md")
                .unwrap()
                .last_updated_at,
            "unchanged doc must yield stable mtime (no false-positive staleness)"
        );
    }

    #[test]
    fn build_pointers_pointer_path_is_repo_relative_not_absolute() {
        let tmp = tempfile::tempdir().unwrap();
        write_doc(tmp.path(), "process-state-machine.md", "# States");
        let pointers = build_team_doctrine_pointers(Some(tmp.path()));
        let p = pointers
            .iter()
            .find(|p| p.path == "docs/process-state-machine.md")
            .unwrap();
        assert!(
            !p.path.starts_with('/') && !p.path.contains(":\\"),
            "pointer path must be repo-relative for portability across machines; got {}",
            p.path
        );
    }
}
