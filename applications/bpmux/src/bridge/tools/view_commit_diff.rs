const DEFAULT_MAX_BYTES: usize = 100_000;
const TRUNCATION_MARKER_TEMPLATE: &str =
    "\n\n--- diff truncated ({shown} of {total} bytes) — request a narrower view ---\n";

pub fn is_safe_ref(sha: &str) -> bool {
    if sha.is_empty() {
        return false;
    }
    sha.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'))
}

pub fn truncate_diff(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }
    let shown_end = max_bytes.min(output.len());
    let safe_end = find_char_boundary(output, shown_end);
    let shown = &output[..safe_end];
    let marker = TRUNCATION_MARKER_TEMPLATE
        .replace("{shown}", &safe_end.to_string())
        .replace("{total}", &output.len().to_string());
    format!("{shown}{marker}")
}

fn find_char_boundary(s: &str, upper: usize) -> usize {
    let mut idx = upper;
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

pub(in crate::bridge) async fn handle_view_commit_diff(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let sha = args
        .get("sha")
        .and_then(|v| v.as_str())
        .ok_or("missing 'sha' argument")?;
    if !is_safe_ref(sha) {
        return Err(format!("invalid sha/ref: {sha}"));
    }
    let max_bytes = args
        .get("max_bytes")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(DEFAULT_MAX_BYTES);
    let raw = run_git_show(sha)?;
    Ok(truncate_diff(&raw, max_bytes))
}

fn run_git_show(sha: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["show", "--format=fuller", "--patch", sha])
        .output()
        .map_err(|e| format!("git show: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "git show exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("git show non-utf8: {e}"))
}
