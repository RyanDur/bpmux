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
