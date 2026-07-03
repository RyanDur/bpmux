pub const GITHUB_BASE_URL: &str = "https://github.com";

pub fn github_actions_url() -> String {
    resolve_ci_actions_url(
        std::env::var("BPMUX_CI_ACTIONS_URL").ok(),
        std::env::var("BPMUX_REPO").ok(),
    )
}

pub(crate) fn resolve_ci_actions_url(configured: Option<String>, repo: Option<String>) -> String {
    let clean = |value: String| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    };
    if let Some(url) = configured.and_then(clean) {
        return url;
    }
    if let Some(repo) = repo.and_then(clean) {
        return format!("{GITHUB_BASE_URL}/{repo}/actions");
    }
    GITHUB_BASE_URL.to_string()
}
