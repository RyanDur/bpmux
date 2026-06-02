use std::path::PathBuf;
use std::process::Command;

pub fn resolve_repo_root() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors() {
            if ancestor.join(".git").exists() {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        for ancestor in cwd.ancestors() {
            if ancestor.join(".git").exists() {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

pub fn current_head_sha() -> Option<String> {
    run_git(&["rev-parse", "HEAD"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn run_git(args: &[&str]) -> Result<String, String> {
    if let Ok(stub_dir) = std::env::var("BPMUX_GIT_STUB_DIR") {
        let subcmd = args.first().copied().unwrap_or("");
        let path = std::path::Path::new(&stub_dir).join(format!("{subcmd}.txt"));
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("git stub {} read failed: {e}", path.display()));
    }
    let mut cmd = Command::new("git");
    if let Some(root) = resolve_repo_root() {
        cmd.current_dir(&root);
    }
    cmd.args(args);
    let output = cmd
        .output()
        .map_err(|e| format!("git {}: {e}", args.first().unwrap_or(&"")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} exited {}: {}",
            args.first().unwrap_or(&""),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("git non-utf8: {e}"))
}
