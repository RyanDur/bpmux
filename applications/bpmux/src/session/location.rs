use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::log::LogError;

pub fn default_sessions_base() -> Result<PathBuf, LogError> {
    if let Ok(override_path) = std::env::var("BPMUX_SESSIONS_BASE")
        && !override_path.is_empty()
    {
        return Ok(PathBuf::from(override_path));
    }
    let base = dirs::home_dir()
        .ok_or_else(|| {
            LogError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found",
            ))
        })?
        .join(".bpmux")
        .join("sessions");
    Ok(base)
}

pub fn resume_or_create_session(base: &Path) -> Result<PathBuf, LogError> {
    let current = base.join("current");
    if current.is_symlink() {
        let target = std::fs::read_link(&current)?;
        if target.is_dir() {
            return Ok(target);
        }
    }
    create_session(base)
}

pub fn create_session(base: &Path) -> Result<PathBuf, LogError> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let session_dir = base.join(&session_id);
    std::fs::create_dir_all(&session_dir)?;
    let current = base.join("current");
    if current.exists() || current.is_symlink() {
        std::fs::remove_file(&current)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&session_dir, &current)?;
    Ok(session_dir)
}

pub const DIR_SESSION_ID_MARKER: &str = ".bpmux-session-id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirSession {
    pub id: String,
    pub name: String,
}

pub fn read_or_create_dir_session(dir: &Path) -> Result<DirSession, LogError> {
    let marker = dir.join(DIR_SESSION_ID_MARKER);
    if let Ok(existing) = std::fs::read_to_string(&marker) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            if let Ok(session) = serde_json::from_str::<DirSession>(trimmed) {
                return Ok(session);
            }
            return Ok(DirSession {
                id: trimmed.to_string(),
                name: dir_basename(dir),
            });
        }
    }
    let session = DirSession {
        id: uuid::Uuid::new_v4().to_string(),
        name: dir_basename(dir),
    };
    std::fs::create_dir_all(dir)?;
    std::fs::write(&marker, serde_json::to_string(&session)?)?;
    Ok(session)
}

fn dir_basename(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "agent".to_string())
}
