use std::path::{Path, PathBuf};

use message_log::LogError;

mod identity;
mod identity_resolution;
pub use identity_resolution::{
    or_register_user_id_from_pin_or_dir, registered_user_id_from_pin_or_dir,
    resolve_or_register_user_id, resolve_registered_user_id,
};
pub mod test_support;
#[cfg(test)]
mod user_id_pin_tests;

#[cfg(test)]
mod location_migration_tests;
#[cfg(test)]
mod location_tests;

pub use identity::{DIR_USER_ID_MARKER, DirUser, register_dir_user, registered_dir_user};
pub use identity::{register_dir_user_in_base, registered_dir_user_in_base};

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
                diag::log_diag(&format!(
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

pub fn durable_data_dir(base: &Path) -> Result<PathBuf, LogError> {
    std::fs::create_dir_all(base)?;
    migrate_legacy_session_dir(base)?;
    Ok(base.to_path_buf())
}

fn migrate_legacy_session_dir(base: &Path) -> Result<(), LogError> {
    let current = base.join("current");
    if !current.is_symlink() {
        return Ok(());
    }
    let legacy = std::fs::read_link(&current)?;
    std::fs::remove_file(&current)?;
    if !legacy.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&legacy)? {
        let entry = entry?;
        let destination = base.join(entry.file_name());
        if destination.exists() {
            diag::log_diag(&format!(
                "legacy session file {} collides with {} — left in place, never merged or deleted",
                entry.path().display(),
                destination.display()
            ));
            continue;
        }
        std::fs::rename(entry.path(), &destination)?;
    }
    if std::fs::read_dir(&legacy)?.next().is_none() {
        std::fs::remove_dir(&legacy)?;
    }
    Ok(())
}
