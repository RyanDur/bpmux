use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::log::LogError;

fn data_base_override() -> Option<PathBuf> {
    let override_path = std::env::var("BPMUX_DATA_BASE").ok()?;
    if override_path.is_empty() {
        return None;
    }
    Some(PathBuf::from(override_path))
}

fn home_bpmux_dir() -> Result<PathBuf, LogError> {
    dirs::home_dir()
        .ok_or_else(|| {
            LogError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "home directory not found",
            ))
        })
        .map(|home| home.join(".bpmux"))
}

pub(crate) fn resolve_base_with_migration(
    override_base: Option<PathBuf>,
    parent: &Path,
) -> Result<PathBuf, LogError> {
    match override_base {
        Some(path) => Ok(path),
        None => {
            migrate_legacy_sessions_base(parent)?;
            Ok(parent.join("data"))
        }
    }
}

pub fn resolve_default_base() -> Result<PathBuf, LogError> {
    resolve_base_with_migration(data_base_override(), &home_bpmux_dir()?)
}

static LEGACY_LEFTOVER_DIAG_ONCE: std::sync::Once = std::sync::Once::new();

pub(crate) fn migrate_legacy_sessions_base(parent: &Path) -> Result<(), LogError> {
    let data = parent.join("data");
    let sessions = parent.join("sessions");
    if data.exists() {
        if sessions.exists() {
            LEGACY_LEFTOVER_DIAG_ONCE.call_once(|| {
                crate::diag::log_diag(&format!(
                    "legacy data base leftover at {} — data/ is authoritative; the leftover is never merged or deleted",
                    sessions.display()
                ));
            });
        }
        return Ok(());
    }
    if !sessions.exists() {
        return Ok(());
    }
    std::fs::rename(&sessions, &data)?;
    repoint_current_symlink(&data, &sessions)
}

fn repoint_current_symlink(data: &Path, old_base: &Path) -> Result<(), LogError> {
    let current = data.join("current");
    if !current.is_symlink() {
        return Ok(());
    }
    let target = std::fs::read_link(&current)?;
    let Ok(suffix) = target.strip_prefix(old_base) else {
        return Ok(());
    };
    let new_target = data.join(suffix);
    std::fs::remove_file(&current)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&new_target, &current)?;
    Ok(())
}

pub fn resume_or_create_data_dir(base: &Path) -> Result<PathBuf, LogError> {
    let current = base.join("current");
    if current.is_symlink() {
        let target = std::fs::read_link(&current)?;
        if target.is_dir() {
            return Ok(target);
        }
    }
    create_data_dir(base)
}

pub fn create_data_dir(base: &Path) -> Result<PathBuf, LogError> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let data_dir = base.join(&session_id);
    std::fs::create_dir_all(&data_dir)?;
    let current = base.join("current");
    if current.exists() || current.is_symlink() {
        std::fs::remove_file(&current)?;
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(&data_dir, &current)?;
    Ok(data_dir)
}

pub const DIR_USER_ID_MARKER: &str = ".bpmux-user-id";
const OLD_DIR_USER_ID_MARKER: &str = ".bpmux-session-id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirUser {
    pub id: String,
    pub name: String,
}

pub fn read_or_create_dir_user(dir: &Path) -> Result<DirUser, LogError> {
    let marker = dir.join(DIR_USER_ID_MARKER);
    if let Ok(existing) = std::fs::read_to_string(&marker)
        && let Some(user) = parse_marker(&existing, dir)
    {
        return Ok(user);
    }
    let old_marker = dir.join(OLD_DIR_USER_ID_MARKER);
    if let Ok(existing) = std::fs::read_to_string(&old_marker)
        && let Some(user) = parse_marker(&existing, dir)
    {
        std::fs::write(&marker, serde_json::to_string(&user)?)?;
        return Ok(user);
    }
    let user = DirUser {
        id: uuid::Uuid::new_v4().to_string(),
        name: dir_basename(dir),
    };
    std::fs::create_dir_all(dir)?;
    std::fs::write(&marker, serde_json::to_string(&user)?)?;
    Ok(user)
}

fn parse_marker(contents: &str, dir: &Path) -> Option<DirUser> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(user) = serde_json::from_str::<DirUser>(trimmed) {
        return Some(user);
    }
    Some(DirUser {
        id: trimmed.to_string(),
        name: dir_basename(dir),
    })
}

fn dir_basename(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "agent".to_string())
}
