#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::git_repo::{resolve_repo_root, run_git};

#[test]
fn resolve_repo_root_walks_up_from_executable_location() {
    let root = resolve_repo_root().expect("should find repo root");
    assert!(
        root.join(".git").exists(),
        "returned root should contain .git: {root:?}"
    );
}

#[test]
fn run_git_resolves_refs_regardless_of_process_cwd() {
    let output = run_git(&["rev-parse", "HEAD"]).expect("git rev-parse HEAD");
    let sha = output.trim();
    assert_eq!(
        sha.len(),
        40,
        "HEAD should resolve to a 40-char sha: got {sha}"
    );
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn run_git_propagates_non_zero_exit_as_err() {
    let result = run_git(&["log", "nonexistent-ref-that-should-fail-loudly"]);
    assert!(result.is_err(), "bad ref should error: {result:?}");
    let err = result.unwrap_err();
    assert!(
        err.contains("exited") || err.contains("unknown revision") || err.contains("fatal"),
        "error should cite git failure: {err}"
    );
}

#[test]
fn run_git_with_show_toplevel_returns_absolute_repo_path() {
    let output = run_git(&["rev-parse", "--show-toplevel"]).expect("toplevel");
    let path = output.trim();
    assert!(std::path::Path::new(path).is_absolute());
    assert!(std::path::Path::new(path).join(".git").exists());
}
