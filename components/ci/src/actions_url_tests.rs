#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::GITHUB_BASE_URL;
use crate::actions_url::resolve_ci_actions_url;

#[test]
fn resolve_ci_actions_url_falls_back_to_github_base_when_unset() {
    assert_eq!(resolve_ci_actions_url(None, None), GITHUB_BASE_URL);
}

#[test]
fn resolve_ci_actions_url_derives_actions_tab_from_repo() {
    assert_eq!(
        resolve_ci_actions_url(None, Some("acme/bpmux".to_string())),
        "https://github.com/acme/bpmux/actions"
    );
}

#[test]
fn resolve_ci_actions_url_honors_configured_override() {
    assert_eq!(
        resolve_ci_actions_url(
            Some("https://example.test/ci".to_string()),
            Some("acme/bpmux".to_string())
        ),
        "https://example.test/ci"
    );
}

#[test]
fn resolve_ci_actions_url_ignores_blank_override_and_derives_from_repo() {
    assert_eq!(
        resolve_ci_actions_url(Some("   ".to_string()), Some("acme/bpmux".to_string())),
        "https://github.com/acme/bpmux/actions"
    );
}
