pub fn parse_acceptance_test_path(body: &str) -> Option<String> {
    let marker = "## Acceptance test path\n\n";
    let start = body.find(marker)? + marker.len();
    let rest = &body[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    let section = &rest[..end];
    for line in section.lines() {
        if let Some(path) = extract_path_from_line(line) {
            return Some(path);
        }
    }
    None
}

fn extract_path_from_line(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let stripped = line
        .trim_start_matches(|c: char| c == '-' || c == '*' || c == '•' || c.is_whitespace())
        .trim();
    let after_label = match stripped.find(": ") {
        Some(idx) => &stripped[idx + 2..],
        None => stripped,
    };
    let trimmed = after_label
        .trim()
        .trim_matches(|c: char| c == '`' || c == '*' || c == '_');
    let token = match trimmed.find([' ', '\t', '(']) {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    let candidate = token
        .trim_end_matches(['.', ',', ';', '`', '*', '_'])
        .trim();
    if candidate.contains('/') && is_acceptable_acceptance_test_path(candidate) {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn is_acceptable_acceptance_test_path(candidate: &str) -> bool {
    const SOURCE_EXTS: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rb", ".java", ".kt", ".cs", ".cpp",
        ".c", ".h", ".hpp", ".swift", ".m",
    ];
    const OPS_EXTS: &[&str] = &[".md", ".yml", ".yaml", ".toml", ".json"];
    const OPS_PREFIXES: &[&str] = &["docs/", ".github/workflows/"];
    if SOURCE_EXTS.iter().any(|ext| candidate.ends_with(ext)) {
        return true;
    }
    OPS_PREFIXES
        .iter()
        .any(|prefix| candidate.starts_with(prefix))
        && OPS_EXTS.iter().any(|ext| candidate.ends_with(ext))
}

fn user_facing_story_type(t: crate::board::domain::StoryType) -> bool {
    matches!(
        t,
        crate::board::domain::StoryType::Feature | crate::board::domain::StoryType::Bug
    )
}

fn check_path_exists(path: &str) -> Result<(), String> {
    if let Some(resolved) = resolve_acceptance_test_path(path) {
        if resolved.exists() {
            return Ok(());
        }
        return Err(format!(
            "acceptance_test_path '{path}' resolved to {resolved:?} but file is missing. Create the test file before pickup OR fix the path. (repo-root: {:?}; cwd: {:?})",
            find_repo_root(),
            std::env::current_dir().ok(),
        ));
    }
    Err(format!(
        "acceptance_test_path '{path}' does not exist; tried absolute, repo-root-relative, and cwd-walk shapes. Create the test file before pickup OR fix the path. (repo-root: {:?}; cwd: {:?})",
        find_repo_root(),
        std::env::current_dir().ok(),
    ))
}

pub(crate) fn resolve_acceptance_test_path(path: &str) -> Option<std::path::PathBuf> {
    let raw = std::path::PathBuf::from(path);
    if raw.is_absolute() {
        return raw.exists().then_some(raw);
    }
    if let Some(root) = find_repo_root() {
        let from_root = root.join(path);
        if from_root.exists() {
            return Some(from_root);
        }
    }
    let cwd_candidates = [
        raw.clone(),
        std::path::PathBuf::from("..").join(path),
        std::path::PathBuf::from("../..").join(path),
    ];
    cwd_candidates.into_iter().find(|p| p.exists())
}

pub(crate) fn find_repo_root() -> Option<std::path::PathBuf> {
    if let Some(start) = std::env::current_dir().ok()
        && let Some(root) = walk_for_git_root(&start)
    {
        return Some(root);
    }
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    walk_for_git_root(&manifest_dir)
}

fn walk_for_git_root(start: &std::path::Path) -> Option<std::path::PathBuf> {
    for ancestor in start.ancestors() {
        if ancestor.join(".git").exists() {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

pub(crate) fn validate_acceptance_test_path_at_pickup(
    args: &serde_json::Map<String, serde_json::Value>,
    number: u64,
    refresh: &dyn crate::board::RefreshDriver,
) -> Result<(), String> {
    let Some(story) = refresh.story(number) else {
        return Ok(());
    };
    if !user_facing_story_type(story.story_type) {
        return Ok(());
    }
    let arg_path = args
        .get("acceptance_test_path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let Some(path) = arg_path.or(story.acceptance_test_path.as_deref()) else {
        return Ok(());
    };
    check_path_exists(path)
}

pub(crate) fn validate_acceptance_test_path_at_deliver(
    number: u64,
    refresh: &dyn crate::board::RefreshDriver,
) -> Result<(), String> {
    let Some(story) = refresh.story(number) else {
        return Ok(());
    };
    if !user_facing_story_type(story.story_type) {
        return Ok(());
    }
    let Some(path) = story.acceptance_test_path.as_deref() else {
        return Ok(());
    };
    check_path_exists(path)
}
