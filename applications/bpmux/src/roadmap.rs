//! Roadmap content access — shared between `terminal` (renders it in
//! Ryan's chat pane) and `bridge` (returns it to agents via the
//! `view_roadmap` MCP tool).
//!
//! Kept below both layers so neither has to reach across into the
//! other. Same pattern as `crate::github` for the GitHub-error
//! taxonomy.

use std::path::Path;

/// Resolve the repo root. Respects `BPMUX_ROOT` override (set by the
/// test harness and the bridge subprocess launcher) before falling
/// back to `git rev-parse`.
pub fn repo_root() -> Result<String, String> {
    if let Ok(root) = std::env::var("BPMUX_ROOT") {
        return Ok(root);
    }
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("failed to find repo root: {e}"))?;
    if !output.status.success() {
        return Err("not inside a git repository".to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Read ROADMAP.md from the repo root. Returns the file's text, or
/// an error string if the file is missing / unreadable.
pub fn fetch_roadmap() -> Result<String, String> {
    let root = repo_root()?;
    let path = Path::new(&root).join("ROADMAP.md");
    std::fs::read_to_string(&path).map_err(|e| format!("cannot read ROADMAP.md: {e}"))
}
