//! Identity layer — #190 / two-layer memory architecture.
//!
//! Stores stable, portable agent identity traits that persist across
//! sessions and project boundaries. Distinct from lessons (which are
//! session-scoped and decay). Identity traits represent WHO the agent
//! is — principles, values, working patterns — not WHAT they learned
//! in a specific session.
//!
//! Schema: values→principles (#269). Each role's identity is a list
//! of values; each value carries a name, a description, and a list
//! of principles that operationalize the value. The on-disk file at
//! `~/.bpmux/identity/{role}.json` uses this shape.
//!
//! Storage: `~/.bpmux/identity/{role}.json`. Cross-session by design;
//! sits alongside `~/.bpmux/sessions/` and `~/.bpmux/tokens/`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One value the agent holds, and the principles that operationalize
/// it. Pre-#269 this was a flat `IdentityTrait` (name + description
/// only); the on-disk files have carried the values+principles shape
/// since day one, so the Rust types caught up to the canonical
/// schema rather than the other way around.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityValue {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub principles: Vec<String>,
}

/// The identity index for one agent role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IdentityIndex {
    pub values: Vec<IdentityValue>,
}

/// Errors from CRUD operations on an IdentityIndex. Narrow enum so
/// handlers can surface a structured JSON error per #269's spec —
/// machine-parseable `kind` + human-readable `detail`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    ValueNotFound(String),
    ValueAlreadyExists(String),
    PrincipleNotFound { value: String, principle: String },
    PrincipleAlreadyExists { value: String, principle: String },
}

impl IdentityError {
    /// Machine-readable error kind for tool responses.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::ValueNotFound(_) => "value_not_found",
            Self::ValueAlreadyExists(_) => "value_already_exists",
            Self::PrincipleNotFound { .. } => "principle_not_found",
            Self::PrincipleAlreadyExists { .. } => "principle_already_exists",
        }
    }
}

impl std::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ValueNotFound(v) => write!(f, "value not found: {v}"),
            Self::ValueAlreadyExists(v) => write!(f, "value already exists: {v}"),
            Self::PrincipleNotFound { value, principle } => {
                write!(f, "principle not found in value '{value}': {principle}")
            }
            Self::PrincipleAlreadyExists { value, principle } => {
                write!(f, "principle already exists in value '{value}': {principle}")
            }
        }
    }
}

impl std::error::Error for IdentityError {}

/// Default identity directory: `~/.bpmux/identity/`.
pub fn default_identity_dir() -> Result<PathBuf, std::io::Error> {
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(home.join(".bpmux").join("identity"))
}

/// Sanitize a name for use in file paths — only alphanumeric, dash,
/// underscore allowed. Prevents path traversal via role/peer names.
pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn identity_path(base: &Path, role: &str) -> PathBuf {
    base.join(format!("{}.json", sanitize_name(role)))
}

/// Load identity for a role. Returns an empty index if the file
/// doesn't exist (first-use case, not an error).
pub fn load_identity(base: &Path, role: &str) -> IdentityIndex {
    let path = identity_path(base, role);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => IdentityIndex::default(),
    }
}

/// Save identity for a role. Creates parent directories if needed.
pub fn save_identity(base: &Path, role: &str, index: &IdentityIndex) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(base)?;
    let path = identity_path(base, role);
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

/// Create a new value with the given name and description. Errors
/// if the value name is already present (no accidental overwrite of
/// description). Principles list starts empty; use `add_principle`
/// to populate.
pub fn add_value(
    index: &mut IdentityIndex,
    name: &str,
    description: &str,
) -> Result<(), IdentityError> {
    if index.values.iter().any(|v| v.name == name) {
        return Err(IdentityError::ValueAlreadyExists(name.to_string()));
    }
    index.values.push(IdentityValue {
        name: name.to_string(),
        description: description.to_string(),
        principles: Vec::new(),
    });
    Ok(())
}

/// Append a principle under the named value. Errors if the value
/// doesn't exist or the principle is already present (dedup).
pub fn add_principle(
    index: &mut IdentityIndex,
    value_name: &str,
    principle: &str,
) -> Result<(), IdentityError> {
    let value = index
        .values
        .iter_mut()
        .find(|v| v.name == value_name)
        .ok_or_else(|| IdentityError::ValueNotFound(value_name.to_string()))?;
    if value.principles.iter().any(|p| p == principle) {
        return Err(IdentityError::PrincipleAlreadyExists {
            value: value_name.to_string(),
            principle: principle.to_string(),
        });
    }
    value.principles.push(principle.to_string());
    Ok(())
}

/// Replace one principle's text with another in the named value.
/// Errors if value or old principle not found.
pub fn update_principle(
    index: &mut IdentityIndex,
    value_name: &str,
    old: &str,
    new: &str,
) -> Result<(), IdentityError> {
    let value = index
        .values
        .iter_mut()
        .find(|v| v.name == value_name)
        .ok_or_else(|| IdentityError::ValueNotFound(value_name.to_string()))?;
    let slot = value
        .principles
        .iter_mut()
        .find(|p| *p == old)
        .ok_or_else(|| IdentityError::PrincipleNotFound {
            value: value_name.to_string(),
            principle: old.to_string(),
        })?;
    *slot = new.to_string();
    Ok(())
}

/// Remove a principle from the named value. Errors if value or
/// principle not found (so the caller knows it was a no-op vs a
/// genuine removal).
pub fn remove_principle(
    index: &mut IdentityIndex,
    value_name: &str,
    principle: &str,
) -> Result<(), IdentityError> {
    let value = index
        .values
        .iter_mut()
        .find(|v| v.name == value_name)
        .ok_or_else(|| IdentityError::ValueNotFound(value_name.to_string()))?;
    let pos = value
        .principles
        .iter()
        .position(|p| p == principle)
        .ok_or_else(|| IdentityError::PrincipleNotFound {
            value: value_name.to_string(),
            principle: principle.to_string(),
        })?;
    value.principles.remove(pos);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn value(name: &str, desc: &str, principles: &[&str]) -> IdentityValue {
        IdentityValue {
            name: name.to_string(),
            description: desc.to_string(),
            principles: principles.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn save_and_load_identity_roundtrips() {
        let dir = tempdir().unwrap();
        let index = IdentityIndex {
            values: vec![
                value(
                    "correctness",
                    "code does what it claims",
                    &["red test first", "verify before claiming"],
                ),
                value(
                    "honesty",
                    "say what's true",
                    &["stay in dev lane"],
                ),
            ],
        };
        save_identity(dir.path(), "dev", &index).unwrap();
        let loaded = load_identity(dir.path(), "dev");
        assert_eq!(loaded, index);
    }

    #[test]
    fn load_identity_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let loaded = load_identity(dir.path(), "dev");
        assert!(loaded.values.is_empty());
    }

    #[test]
    fn per_role_isolation() {
        let dir = tempdir().unwrap();
        let dev_index = IdentityIndex {
            values: vec![value("dev value", "dev-specific", &["p1"])],
        };
        let pm_index = IdentityIndex {
            values: vec![value("pm value", "pm-specific", &["q1"])],
        };
        save_identity(dir.path(), "dev", &dev_index).unwrap();
        save_identity(dir.path(), "pm", &pm_index).unwrap();

        let dev_loaded = load_identity(dir.path(), "dev");
        let pm_loaded = load_identity(dir.path(), "pm");
        assert_eq!(dev_loaded.values[0].name, "dev value");
        assert_eq!(pm_loaded.values[0].name, "pm value");
        assert_ne!(dev_loaded, pm_loaded);
    }

    #[test]
    fn identity_path_is_separate_from_session_path() {
        let identity_base = PathBuf::from("/home/user/.bpmux/identity");
        let session_base = PathBuf::from("/home/user/.bpmux/sessions/some-uuid");
        let identity_file = identity_path(&identity_base, "dev");
        assert!(!identity_file.starts_with(&session_base));
        assert!(identity_file.starts_with(&identity_base));
    }

    #[test]
    fn default_identity_dir_is_under_bpmux() {
        let dir = default_identity_dir().unwrap();
        let s = dir.to_string_lossy();
        assert!(s.contains(".bpmux"), "expected .bpmux in path, got {s}");
        assert!(s.ends_with("identity"), "expected identity suffix, got {s}");
    }

    #[test]
    fn add_principle_appends_to_value() {
        let mut idx = IdentityIndex {
            values: vec![value("correctness", "", &["red test first"])],
        };
        add_principle(&mut idx, "correctness", "verify before claiming").unwrap();
        assert_eq!(idx.values[0].principles.len(), 2);
        assert_eq!(idx.values[0].principles[1], "verify before claiming");
    }

    #[test]
    fn add_principle_rejects_duplicate() {
        let mut idx = IdentityIndex {
            values: vec![value("correctness", "", &["red test first"])],
        };
        let err =
            add_principle(&mut idx, "correctness", "red test first").unwrap_err();
        assert!(matches!(err, IdentityError::PrincipleAlreadyExists { .. }));
    }

    #[test]
    fn add_principle_rejects_unknown_value() {
        let mut idx = IdentityIndex { values: vec![] };
        let err = add_principle(&mut idx, "nonexistent", "p").unwrap_err();
        assert!(matches!(err, IdentityError::ValueNotFound(_)));
    }

    #[test]
    fn update_principle_replaces_text() {
        let mut idx = IdentityIndex {
            values: vec![value("honesty", "", &["old phrasing"])],
        };
        update_principle(&mut idx, "honesty", "old phrasing", "refined phrasing").unwrap();
        assert_eq!(idx.values[0].principles[0], "refined phrasing");
    }

    #[test]
    fn update_principle_rejects_missing_principle() {
        let mut idx = IdentityIndex {
            values: vec![value("honesty", "", &["something else"])],
        };
        let err = update_principle(&mut idx, "honesty", "missing", "new").unwrap_err();
        assert!(matches!(err, IdentityError::PrincipleNotFound { .. }));
    }

    #[test]
    fn remove_principle_drops_from_list() {
        let mut idx = IdentityIndex {
            values: vec![value("discipline", "", &["keep", "drop", "retain"])],
        };
        remove_principle(&mut idx, "discipline", "drop").unwrap();
        assert_eq!(idx.values[0].principles, vec!["keep", "retain"]);
    }

    #[test]
    fn remove_principle_rejects_missing() {
        let mut idx = IdentityIndex {
            values: vec![value("discipline", "", &["keep"])],
        };
        let err = remove_principle(&mut idx, "discipline", "gone").unwrap_err();
        assert!(matches!(err, IdentityError::PrincipleNotFound { .. }));
    }

    #[test]
    fn add_value_appends_empty_value() {
        let mut idx = IdentityIndex { values: vec![] };
        add_value(&mut idx, "correctness", "code does what it claims").unwrap();
        assert_eq!(idx.values.len(), 1);
        assert_eq!(idx.values[0].name, "correctness");
        assert!(idx.values[0].principles.is_empty());
    }

    #[test]
    fn add_value_rejects_duplicate_name() {
        let mut idx = IdentityIndex {
            values: vec![value("correctness", "existing", &[])],
        };
        let err = add_value(&mut idx, "correctness", "would overwrite").unwrap_err();
        assert!(matches!(err, IdentityError::ValueAlreadyExists(_)));
        assert_eq!(idx.values[0].description, "existing");
    }

    #[test]
    fn load_identity_parses_real_file_schema() {
        // Pre-#269 regression: load_identity used to return an
        // empty IdentityIndex for real files because IdentityIndex
        // had a `traits` field but the on-disk schema has `values`.
        // This test pins the schema fix — the JSON shape matches
        // what actually lives in ~/.bpmux/identity/*.json.
        let dir = tempdir().unwrap();
        let path = dir.path().join("dev.json");
        std::fs::write(
            &path,
            r#"{
                "values": [
                    {
                        "name": "correctness",
                        "description": "code does what it claims",
                        "principles": ["red test first", "verify before claiming"]
                    }
                ]
            }"#,
        )
        .unwrap();
        let loaded = load_identity(dir.path(), "dev");
        assert_eq!(loaded.values.len(), 1);
        assert_eq!(loaded.values[0].name, "correctness");
        assert_eq!(loaded.values[0].principles.len(), 2);
    }
}
