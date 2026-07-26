#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::actions_url::ActionsUrl;

#[test]
fn a_connected_repo_yields_its_actions_tab() {
    assert_eq!(
        ActionsUrl::for_repo(Some(("acme", "bpmux"))).as_str(),
        "https://github.com/acme/bpmux/actions"
    );
}

#[test]
fn no_connection_falls_back_to_the_github_base() {
    assert_eq!(ActionsUrl::for_repo(None).as_str(), "https://github.com");
}
