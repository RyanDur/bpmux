use crate::location;

pub(super) fn resolve_data_base(cli_base: &Option<String>) -> anyhow::Result<std::path::PathBuf> {
    match cli_base {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => Ok(location::resolve_default_base()?),
    }
}

pub(crate) fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(local_url::DEFAULT_API_PORT)
}

pub(crate) fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}

pub(crate) fn resolve_registered_user_id(cwd: &std::path::Path) -> Option<String> {
    let base = location::resolve_default_base().ok()?;
    registered_user_id_from_pin_or_dir(std::env::var("BPMUX_USER_ID").ok(), &base, cwd)
}

pub(crate) fn resolve_or_register_user_id(cwd: &std::path::Path) -> anyhow::Result<String> {
    let base = location::resolve_default_base()?;
    or_register_user_id_from_pin_or_dir(std::env::var("BPMUX_USER_ID").ok(), &base, cwd)
}

fn pinned_id(pinned: Option<String>) -> Option<String> {
    pinned
        .map(|pin| pin.trim().to_string())
        .filter(|pin| !pin.is_empty())
}

pub(crate) fn registered_user_id_from_pin_or_dir(
    pinned: Option<String>,
    base: &std::path::Path,
    cwd: &std::path::Path,
) -> Option<String> {
    if let Some(pin) = pinned_id(pinned) {
        return Some(pin);
    }
    location::registered_dir_user_in_base(base, cwd).map(|user| user.id)
}

pub(crate) fn or_register_user_id_from_pin_or_dir(
    pinned: Option<String>,
    base: &std::path::Path,
    cwd: &std::path::Path,
) -> anyhow::Result<String> {
    if let Some(pin) = pinned_id(pinned) {
        return Ok(pin);
    }
    Ok(location::register_dir_user_in_base(base, cwd)?.id)
}
