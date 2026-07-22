use std::path::PathBuf;

pub const MAX_BRIEFING_BYTES: usize = 1024 * 1024;

pub fn briefings_dir() -> PathBuf {
    if let Some(override_path) = std::env::var_os("BPMUX_BRIEFINGS_DIR") {
        return PathBuf::from(override_path);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join(".bpmux").join("briefings")
}

pub fn briefing_path(role: &str) -> PathBuf {
    briefing_path_in(&briefings_dir(), role)
}

pub fn briefing_path_in(root: &std::path::Path, role: &str) -> PathBuf {
    let safe_role = memory::identity::sanitize_name(role);
    root.join(format!("{safe_role}.md"))
}

pub fn load_briefing(role: &str) -> String {
    load_briefing_in(&briefings_dir(), role)
}

pub fn load_briefing_in(root: &std::path::Path, role: &str) -> String {
    let path = briefing_path_in(root, role);
    std::fs::read_to_string(&path).unwrap_or_default()
}

pub fn save_briefing(role: &str, content: &str) -> Result<(), std::io::Error> {
    save_briefing_in(&briefings_dir(), role, content)
}

pub fn save_briefing_in(
    root: &std::path::Path,
    role: &str,
    content: &str,
) -> Result<(), std::io::Error> {
    if content.len() > MAX_BRIEFING_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "briefing content exceeds {MAX_BRIEFING_BYTES}-byte cap \
                 ({} bytes)",
                content.len()
            ),
        ));
    }
    std::fs::create_dir_all(root)?;
    let path = briefing_path_in(root, role);
    std::fs::write(&path, content)
}

#[cfg(test)]
mod briefing_tests;
