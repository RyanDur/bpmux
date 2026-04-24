use super::git_repo::run_git;

const FIELD_DELIMITER: &str = "\x1f";
const GIT_FORMAT: &str = "%H\x1f%h\x1f%s\x1f%an\x1f%aI";

pub fn subject_matches_story(subject: &str, number: u64) -> bool {
    let token = format!("#{number}");
    let bytes = subject.as_bytes();
    let needle = token.as_bytes();
    for start in 0..bytes.len().saturating_sub(needle.len() - 1) {
        if !bytes[start..].starts_with(needle) {
            continue;
        }
        let after = start + needle.len();
        let next_is_boundary = bytes
            .get(after)
            .map(|b| !b.is_ascii_digit())
            .unwrap_or(true);
        if next_is_boundary {
            return true;
        }
    }
    false
}

pub fn parse_git_log_output(text: &str, number: u64) -> Vec<serde_json::Value> {
    text.lines()
        .filter_map(|line| parse_line(line, number))
        .collect()
}

fn parse_line(line: &str, number: u64) -> Option<serde_json::Value> {
    let parts: Vec<&str> = line.splitn(5, FIELD_DELIMITER).collect();
    if parts.len() != 5 {
        return None;
    }
    let subject = parts[2];
    if !subject_matches_story(subject, number) {
        return None;
    }
    Some(serde_json::json!({
        "sha": parts[0],
        "short_sha": parts[1],
        "subject": subject,
        "author": parts[3],
        "committed_at": parts[4],
    }))
}

pub(in crate::bridge) async fn handle_view_story_commits(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let format_arg = format!("--format={GIT_FORMAT}");
    let output = run_git(&["log", "main", &format_arg])?;
    let commits = parse_git_log_output(&output, number);
    serde_json::to_string_pretty(&commits).map_err(|e| format!("serialize commits: {e}"))
}
