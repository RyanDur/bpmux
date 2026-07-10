use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::log::LogError;

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

pub(crate) fn registered_dir_user_in_base(base: &Path, dir: &Path) -> Option<DirUser> {
    if let Some(user) = read_entry(base, dir) {
        return Some(user);
    }
    import_marker(base, dir)
}

pub(crate) fn register_dir_user_in_base(base: &Path, dir: &Path) -> Result<DirUser, LogError> {
    if let Some(user) = registered_dir_user_in_base(base, dir) {
        return Ok(user);
    }
    mint_entry(base, dir)
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

fn mint_entry(base: &Path, dir: &Path) -> Result<DirUser, LogError> {
    let (path, canonical) = entry_path(base, dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let user = DirUser {
        id: uuid::Uuid::new_v4().to_string(),
        name: dir_basename(dir),
    };
    let entry = RegistryEntry {
        path: canonical,
        id: user.id.clone(),
        name: user.name.clone(),
    };
    match std::fs::File::create_new(&path) {
        Ok(mut file) => {
            file.write_all(serde_json::to_string(&entry)?.as_bytes())?;
            Ok(user)
        }
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => read_entry(base, dir)
            .ok_or_else(|| {
                LogError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "registry entry vanished after a create_new race",
                ))
            }),
        Err(err) => Err(LogError::Io(err)),
    }
}

fn import_marker(base: &Path, dir: &Path) -> Option<DirUser> {
    let new_marker = dir.join(DIR_USER_ID_MARKER);
    let old_marker = dir.join(OLD_DIR_USER_ID_MARKER);
    let from_new = read_marker(&new_marker, dir);
    let from_old = read_marker(&old_marker, dir);
    let user = match (from_new, from_old) {
        (Some(new_user), Some(old_user)) => {
            if new_user.id != old_user.id {
                log_marker_divergence(dir);
            }
            new_user
        }
        (Some(new_user), None) => new_user,
        (None, Some(old_user)) => old_user,
        (None, None) => return None,
    };
    write_entry(base, dir, &user).ok()?;
    let _ = std::fs::remove_file(&new_marker);
    let _ = std::fs::remove_file(&old_marker);
    Some(user)
}

fn write_entry(base: &Path, dir: &Path, user: &DirUser) -> Result<(), LogError> {
    let (path, canonical) = entry_path(base, dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let entry = RegistryEntry {
        path: canonical,
        id: user.id.clone(),
        name: user.name.clone(),
    };
    std::fs::write(&path, serde_json::to_string(&entry)?)?;
    Ok(())
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
        crate::diag::log_diag(&format!(
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
