use crate::{location, transport};

pub(super) fn resolve_session_base(
    cli_base: &Option<String>,
) -> anyhow::Result<std::path::PathBuf> {
    match cli_base {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => match std::env::var("BPMUX_SESSION_BASE") {
            Ok(path) => Ok(std::path::PathBuf::from(path)),
            Err(_) => Ok(location::default_data_base()?),
        },
    }
}

pub(crate) fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(transport::local_url::DEFAULT_API_PORT)
}

pub(crate) fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}

pub(crate) fn resolve_user_id(cwd: &std::path::Path) -> anyhow::Result<String> {
    user_id_from_pin_or_dir(std::env::var("BPMUX_USER_ID").ok(), cwd)
}

pub(crate) fn user_id_from_pin_or_dir(
    pinned: Option<String>,
    cwd: &std::path::Path,
) -> anyhow::Result<String> {
    if let Some(pin) = pinned {
        let pin = pin.trim();
        if !pin.is_empty() {
            return Ok(pin.to_string());
        }
    }
    Ok(location::read_or_create_dir_user(cwd)?.id)
}
