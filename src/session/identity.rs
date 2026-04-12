//! Identity layer — #190 / two-layer memory architecture.
//!
//! Stores stable, portable agent identity traits that persist across
//! sessions and project boundaries. Distinct from lessons (which are
//! session-scoped and decay). Identity traits represent WHO the agent
//! is — principles, values, working patterns — not WHAT they learned
//! in a specific session.
//!
//! Storage: `~/.bpmux/identity/{role}.json`. Cross-session by design;
//! sits alongside `~/.bpmux/sessions/` and `~/.bpmux/tokens/`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One identity trait. Kept simple for v1 — just a name and
/// description. Future slices add reinforcement tracking, tension
/// pairs, graduation provenance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityTrait {
    pub name: String,
    pub description: String,
}

/// The identity index for one agent role.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IdentityIndex {
    pub traits: Vec<IdentityTrait>,
}

/// Default identity directory: `~/.bpmux/identity/`.
pub fn default_identity_dir() -> Result<PathBuf, std::io::Error> {
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(home.join(".bpmux").join("identity"))
}

/// Path to a specific role's identity file within a base directory.
pub fn identity_path(base: &Path, role: &str) -> PathBuf {
    base.join(format!("{role}.json"))
}

/// Load identity traits for a role. Returns an empty index if the
/// file doesn't exist (first-use case, not an error).
pub fn load_identity(base: &Path, role: &str) -> IdentityIndex {
    let path = identity_path(base, role);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => IdentityIndex::default(),
    }
}

/// Save identity traits for a role. Creates parent directories if
/// needed.
pub fn save_identity(base: &Path, role: &str, index: &IdentityIndex) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(base)?;
    let path = identity_path(base, role);
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn trait_entry(name: &str, desc: &str) -> IdentityTrait {
        IdentityTrait {
            name: name.to_string(),
            description: desc.to_string(),
        }
    }

    #[test]
    fn save_and_load_identity_roundtrips() {
        let dir = tempdir().unwrap();
        let index = IdentityIndex {
            traits: vec![
                trait_entry(
                    "verify before claiming",
                    "observable evidence, not tool-return-implied",
                ),
                trait_entry(
                    "stay in dev lane",
                    "feasibility and constraints, not product direction",
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
        assert!(loaded.traits.is_empty());
    }

    #[test]
    fn per_role_isolation() {
        let dir = tempdir().unwrap();
        let dev_index = IdentityIndex {
            traits: vec![trait_entry("dev trait", "dev-specific")],
        };
        let pm_index = IdentityIndex {
            traits: vec![trait_entry("pm trait", "pm-specific")],
        };
        save_identity(dir.path(), "dev", &dev_index).unwrap();
        save_identity(dir.path(), "pm", &pm_index).unwrap();

        let dev_loaded = load_identity(dir.path(), "dev");
        let pm_loaded = load_identity(dir.path(), "pm");
        assert_eq!(dev_loaded.traits[0].name, "dev trait");
        assert_eq!(pm_loaded.traits[0].name, "pm trait");
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
}
