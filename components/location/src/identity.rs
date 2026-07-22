use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use message_log::LogError;

pub const DIR_USER_ID_MARKER: &str = ".bpmux-user-id";
const OLD_DIR_USER_ID_MARKER: &str = ".bpmux-session-id";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirUser {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryEntry {
    path: String,
    id: String,
    name: String,
}

pub fn registered_dir_user(dir: &Path) -> Option<DirUser> {
    let base = super::resolve_default_base().ok()?;
    registered_dir_user_in_base(&base, dir)
}

pub fn register_dir_user(dir: &Path) -> Result<DirUser, LogError> {
    let base = super::resolve_default_base()?;
    register_dir_user_in_base(&base, dir)
}

pub fn registered_dir_user_in_base(base: &Path, dir: &Path) -> Option<DirUser> {
    if let Some(user) = resolve_marker(dir) {
        return Some(user);
    }
    import_central_entry(base, dir)
}

pub fn register_dir_user_in_base(base: &Path, dir: &Path) -> Result<DirUser, LogError> {
    if let Some(user) = registered_dir_user_in_base(base, dir) {
        return Ok(user);
    }
    mint_marker(dir)
}

fn entry_path(base: &Path, dir: &Path) -> (PathBuf, String) {
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let canonical_str = canonical.to_string_lossy().into_owned();
    let key = sha256_hex(canonical_str.as_bytes());
    let path = base
        .join("users")
        .join("by-dir")
        .join(format!("{key}.json"));
    (path, canonical_str)
}

fn read_entry(base: &Path, dir: &Path) -> Option<DirUser> {
    let (path, _) = entry_path(base, dir);
    let raw = std::fs::read_to_string(&path).ok()?;
    let entry = serde_json::from_str::<RegistryEntry>(raw.trim()).ok()?;
    Some(DirUser {
        id: entry.id,
        name: entry.name,
    })
}

fn mint_marker(dir: &Path) -> Result<DirUser, LogError> {
    let user = DirUser {
        id: uuid::Uuid::new_v4().to_string(),
        name: dir_basename(dir),
    };
    match publish_marker(dir, &user)? {
        Some(winner) => Ok(winner),
        None => Ok(user),
    }
}

fn publish_marker(dir: &Path, user: &DirUser) -> Result<Option<DirUser>, LogError> {
    let marker = dir.join(DIR_USER_ID_MARKER);
    let staged = staged_sibling(&marker);
    std::fs::write(&staged, serde_json::to_string(user)?)?;
    let published = std::fs::hard_link(&staged, &marker);
    let _ = std::fs::remove_file(&staged);
    match published {
        Ok(()) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
            read_marker(&marker, dir).map(Some).ok_or_else(|| {
                LogError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "identity marker vanished after a mint race",
                ))
            })
        }
        Err(err) => Err(LogError::Io(err)),
    }
}

fn staged_sibling(path: &Path) -> PathBuf {
    let mut staged = path.as_os_str().to_owned();
    staged.push(format!(".staged-{}", uuid::Uuid::new_v4()));
    PathBuf::from(staged)
}

fn resolve_marker(dir: &Path) -> Option<DirUser> {
    let new_marker = dir.join(DIR_USER_ID_MARKER);
    let old_marker = dir.join(OLD_DIR_USER_ID_MARKER);
    let from_new = read_marker(&new_marker, dir);
    let from_old = read_marker(&old_marker, dir);
    match (from_new, from_old) {
        (Some(new_user), Some(old_user)) => {
            if new_user.id != old_user.id {
                log_marker_divergence(dir);
            }
            let _ = std::fs::remove_file(&old_marker);
            Some(new_user)
        }
        (Some(new_user), None) => Some(new_user),
        (None, Some(old_user)) => {
            let settled = publish_marker(dir, &old_user).ok()?.unwrap_or(old_user);
            let _ = std::fs::remove_file(&old_marker);
            Some(settled)
        }
        (None, None) => None,
    }
}

fn import_central_entry(base: &Path, dir: &Path) -> Option<DirUser> {
    let (entry_path, _) = entry_path(base, dir);
    let user = read_entry(base, dir)?;
    let settled = publish_marker(dir, &user).ok()?.unwrap_or(user);
    let _ = std::fs::remove_file(&entry_path);
    Some(settled)
}

fn read_marker(marker: &Path, dir: &Path) -> Option<DirUser> {
    let contents = std::fs::read_to_string(marker).ok()?;
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

fn log_marker_divergence(dir: &Path) {
    static DIVERGENCE_ONCE: std::sync::Once = std::sync::Once::new();
    DIVERGENCE_ONCE.call_once(|| {
        diag::log_diag(&format!(
            "divergent identity markers in {} — the new .bpmux-user-id is authoritative; the legacy .bpmux-session-id is discarded",
            dir.display()
        ));
    });
}

fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes)
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn dir_basename(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "agent".to_string())
}
