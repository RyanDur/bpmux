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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_repo_holding_the_binary_not_an_unrelated_sibling_repo() {
        let bpmux = tempfile::tempdir().unwrap();
        std::fs::create_dir(bpmux.path().join(".git")).unwrap();
        std::fs::write(bpmux.path().join("ROADMAP.md"), "BPMUX-ROADMAP").unwrap();
        let binary_under_bpmux = bpmux.path().join("target/release/bpmux");

        let sibling_workspace = tempfile::tempdir().unwrap();
        std::fs::create_dir(sibling_workspace.path().join(".git")).unwrap();
        std::fs::write(sibling_workspace.path().join("ROADMAP.md"), "WRONG-ROADMAP").unwrap();

        let resolved = resolve_root(None, Some(binary_under_bpmux))
            .expect("resolves from the binary location");
        assert_eq!(
            resolved,
            bpmux.path(),
            "must resolve the repo CONTAINING the binary, not the sibling workspace repo — the role-workspace is a SIBLING of bpmux (not an ancestor), so a cwd-walk-up would never reach bpmux"
        );
        let content = std::fs::read_to_string(resolved.join("ROADMAP.md")).unwrap();
        assert_eq!(content, "BPMUX-ROADMAP");
    }

    #[test]
    fn bpmux_root_env_overrides_the_binary_walk_up() {
        let explicit = tempfile::tempdir().unwrap();
        let resolved = resolve_root(
            Some(explicit.path().to_string_lossy().to_string()),
            Some(PathBuf::from("/elsewhere/target/release/bpmux")),
        );
        assert_eq!(resolved.as_deref(), Some(explicit.path()));
    }

    #[test]
    fn an_empty_bpmux_root_falls_through_to_the_binary_walk_up() {
        let bpmux = tempfile::tempdir().unwrap();
        std::fs::create_dir(bpmux.path().join(".git")).unwrap();
        let binary_under_bpmux = bpmux.path().join("target/debug/bpmux");

        let resolved = resolve_root(Some(String::new()), Some(binary_under_bpmux));
        assert_eq!(
            resolved.as_deref(),
            Some(bpmux.path()),
            "an empty BPMUX_ROOT must not short-circuit to an empty path — fall through to the binary location"
        );
    }

    #[test]
    fn no_git_root_above_the_binary_yields_none() {
        let bare = tempfile::tempdir().unwrap();
        let binary = bare.path().join("nested/bin/bpmux");
        assert_eq!(resolve_root(None, Some(binary)), None);
    }
}
