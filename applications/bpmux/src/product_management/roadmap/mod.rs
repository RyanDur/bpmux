pub mod drill;
pub mod filter;
pub mod view;

#[cfg(test)]
mod drill_tests;
#[cfg(test)]
mod filter_tests;
#[cfg(test)]
mod repo_root_resolution;
#[cfg(test)]
mod view_tests;

use std::path::{Path, PathBuf};

fn resolve_root(bpmux_root_env: Option<String>, executable: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(root) = bpmux_root_env.filter(|root| !root.is_empty()) {
        return Some(PathBuf::from(root));
    }
    executable?
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

pub fn repo_root() -> Result<String, String> {
    resolve_root(
        std::env::var("BPMUX_ROOT").ok(),
        std::env::current_exe().ok(),
    )
    .map(|root| root.to_string_lossy().to_string())
    .ok_or_else(|| {
        "could not locate the bpmux repo root from the binary location (set BPMUX_ROOT)".to_string()
    })
}

pub fn fetch_roadmap() -> Result<String, String> {
    let root = repo_root()?;
    let path = Path::new(&root).join("ROADMAP.md");
    std::fs::read_to_string(&path).map_err(|e| format!("cannot read ROADMAP.md: {e}"))
}
