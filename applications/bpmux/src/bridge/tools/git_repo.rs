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

pub fn run_git(args: &[&str]) -> Result<String, String> {
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
