use std::path::Path;

use message_log::LogError;

use crate::{register_dir_user_in_base, registered_dir_user_in_base, resolve_default_base};

pub fn resolve_registered_user_id(cwd: &Path) -> Option<String> {
    let base = resolve_default_base().ok()?;
    registered_user_id_from_pin_or_dir(std::env::var("BPMUX_USER_ID").ok(), &base, cwd)
}

pub fn resolve_or_register_user_id(cwd: &Path) -> Result<String, LogError> {
    let base = resolve_default_base()?;
    or_register_user_id_from_pin_or_dir(std::env::var("BPMUX_USER_ID").ok(), &base, cwd)
}

fn pinned_id(pinned: Option<String>) -> Option<String> {
    pinned
        .map(|pin| pin.trim().to_string())
        .filter(|pin| !pin.is_empty())
}

pub fn registered_user_id_from_pin_or_dir(
    pinned: Option<String>,
    base: &Path,
    cwd: &Path,
) -> Option<String> {
    if let Some(pin) = pinned_id(pinned) {
        return Some(pin);
    }
    registered_dir_user_in_base(base, cwd).map(|user| user.id)
}

pub fn or_register_user_id_from_pin_or_dir(
    pinned: Option<String>,
    base: &Path,
    cwd: &Path,
) -> Result<String, LogError> {
    if let Some(pin) = pinned_id(pinned) {
        return Ok(pin);
    }
    Ok(register_dir_user_in_base(base, cwd)?.id)
}
