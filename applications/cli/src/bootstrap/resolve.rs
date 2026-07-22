use location;

pub fn resolve_data_base(cli_base: &Option<String>) -> anyhow::Result<std::path::PathBuf> {
    match cli_base {
        Some(path) => Ok(std::path::PathBuf::from(path)),
        None => Ok(location::resolve_default_base()?),
    }
}

pub fn resolve_bpmux_port() -> u16 {
    std::env::var("BPMUX_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(local_url::DEFAULT_API_PORT)
}

pub fn resolve_role(role_arg: &Option<String>) -> String {
    role_arg
        .clone()
        .or_else(|| std::env::var("BPMUX_ROLE").ok())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "dev".to_string())
}

pub use location::{resolve_or_register_user_id, resolve_registered_user_id};
