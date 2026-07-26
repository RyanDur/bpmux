#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::actions_url::ActionsUrl;

#[test]
fn a_connected_repo_yields_its_actions_tab() {
    assert_eq!(
        ActionsUrl::for_repo(Some(("acme", "bpmux"))).current(),
        "https://github.com/acme/bpmux/actions"
    );
}

#[test]
fn no_connection_falls_back_to_the_github_base() {
    assert_eq!(ActionsUrl::for_repo(None).current(), "https://github.com");
}

#[test]
fn a_connection_change_after_boot_reaches_subsequent_reads() {
    let url = ActionsUrl::for_repo(None);
    url.set_repo(Some(("acme", "bpmux")));
    assert_eq!(
        url.current(),
        "https://github.com/acme/bpmux/actions",
        "connecting after boot re-resolves the url — no restart required"
    );
}

#[test]
fn every_holder_of_the_handle_sees_the_change() {
    let fed_once = ActionsUrl::for_repo(Some(("acme", "bpmux")));
    let downstream = fed_once.clone();
    fed_once.set_repo(Some(("acme", "sand")));
    assert_eq!(
        downstream.current(),
        "https://github.com/acme/sand/actions",
        "clones share the one cell — feed one place, every consumer resolves the new value"
    );
}

#[test]
fn disconnecting_falls_back_to_the_github_base() {
    let url = ActionsUrl::for_repo(Some(("acme", "bpmux")));
    url.set_repo(None);
    assert_eq!(url.current(), "https://github.com");
}
