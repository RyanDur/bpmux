use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityValue {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub principles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct IdentityIndex {
    pub values: Vec<IdentityValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityError {
    ValueNotFound(String),
    ValueAlreadyExists(String),
    PrincipleNotFound { value: String, principle: String },
    PrincipleAlreadyExists { value: String, principle: String },
}

impl IdentityError {
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
                write!(
                    f,
                    "principle already exists in value '{value}': {principle}"
                )
            }
        }
    }
}

impl std::error::Error for IdentityError {}

pub fn default_identity_dir() -> Result<PathBuf, std::io::Error> {
    if let Some(override_path) = std::env::var_os("BPMUX_IDENTITY_DIR") {
        return Ok(PathBuf::from(override_path));
    }
    let home = dirs::home_dir().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "home directory not found")
    })?;
    Ok(home.join(".bpmux").join("identity"))
}

pub fn sanitize_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn identity_path(base: &Path, role: &str) -> PathBuf {
    base.join(format!("{}.json", sanitize_name(role)))
}

pub fn load_identity(base: &Path, role: &str) -> IdentityIndex {
    let path = identity_path(base, role);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => IdentityIndex::default(),
    }
}

pub fn save_identity(base: &Path, role: &str, index: &IdentityIndex) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(base)?;
    let path = identity_path(base, role);
    let json = serde_json::to_string_pretty(index)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

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
                value("honesty", "say what's true", &["stay in dev lane"]),
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
        if std::env::var_os("BPMUX_IDENTITY_DIR").is_some() {
            return;
        }
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
        let err = add_principle(&mut idx, "correctness", "red test first").unwrap_err();
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

    #[test]
    fn all_three_agent_file_shapes_parse() {
        let dir = tempdir().unwrap();

        let dev_fixture = r#"{
            "values": [
                { "name": "correctness", "description": "code does what it claims", "principles": ["p1", "p2", "p3", "p4"] },
                { "name": "honesty", "description": "say what's true", "principles": ["p1", "p2", "p3", "p4", "p5"] },
                { "name": "simplicity", "description": "minimum code", "principles": ["p1", "p2", "p3", "p4"] },
                { "name": "discipline", "description": "checks as set", "principles": ["p1", "p2", "p3", "p4", "p5", "p6"] }
            ]
        }"#;
        std::fs::write(dir.path().join("dev.json"), dev_fixture).unwrap();

        let pm_fixture = r#"{
            "values": [
                { "name": "trust", "description": "foundations", "principles": ["p1", "p2", "p3", "p4", "p5", "p6"] },
                { "name": "communication", "description": "surface what matters", "principles": ["p1", "p2", "p3"] },
                { "name": "experience", "description": "user journey", "principles": ["p1", "p2", "p3"] },
                { "name": "stewardship", "description": "board ownership", "principles": ["p1", "p2"] }
            ]
        }"#;
        std::fs::write(dir.path().join("pm.json"), pm_fixture).unwrap();

        let designer_fixture = r#"{
            "values": [
                { "name": "clarity", "description": "user understands", "principles": ["p1", "p2"] },
                { "name": "honesty", "description": "name the problem", "principles": ["p1", "p2", "p3", "p4"] },
                { "name": "craft", "description": "detail matters", "principles": ["p1", "p2", "p3", "p4"] },
                { "name": "individuality", "description": "diversity", "principles": ["p1", "p2"] }
            ]
        }"#;
        std::fs::write(dir.path().join("designer.json"), designer_fixture).unwrap();

        for role in &["dev", "pm", "designer"] {
            let loaded = load_identity(dir.path(), role);
            assert!(
                !loaded.values.is_empty(),
                "{} identity must load with non-empty values; canonical schema regression if this fails",
                role
            );
            for v in &loaded.values {
                assert!(
                    !v.principles.is_empty(),
                    "{} value '{}' should have principles (canonical schema)",
                    role,
                    v.name
                );
            }
        }
    }

    #[test]
    #[ignore = "loads real ~/.bpmux/identity/*.json — run with --ignored on machines that have them"]
    fn real_identity_files_on_disk_parse_canonical() {
        let dir = match default_identity_dir() {
            Ok(d) => d,
            Err(_) => panic!("no home dir available; can't locate identity files"),
        };
        assert!(
            dir.exists(),
            "{} does not exist; skip this test or populate identity files",
            dir.display()
        );
        for role in &["dev", "pm", "designer"] {
            let loaded = load_identity(&dir, role);
            assert!(
                !loaded.values.is_empty(),
                "{}/{}.json parsed to empty — canonical schema regression",
                dir.display(),
                role
            );
        }
    }
}
