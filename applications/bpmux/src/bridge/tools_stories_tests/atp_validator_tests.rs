#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::board::domain::StoryType;
use crate::product_management::board::domain::story_type_from_labels_or_title;
use crate::transport::protocol::ClientMessage;

#[cfg(test)]
mod atp_validator_repo_root_tests {
    use super::*;

    #[test]
    fn find_repo_root_locates_dot_git_ancestor() {
        let root = find_repo_root().expect("should find repo root from any cwd inside the repo");
        assert!(
            root.join(".git").exists(),
            "find_repo_root must return a directory containing .git: {root:?}"
        );
    }

    #[test]
    fn resolve_repo_root_relative_path_succeeds_regardless_of_cwd() {
        let resolved = resolve_acceptance_test_path(".github/workflows/ci.yml");
        assert!(
            resolved.is_some(),
            "repo-root-relative ATP must resolve regardless of cwd; got None"
        );
        let path = resolved.unwrap();
        assert!(path.exists(), "resolved path must exist: {path:?}");
    }

    #[test]
    fn resolve_absolute_path_passes_through_when_exists() {
        let root = find_repo_root().expect("repo root");
        let abs = root.join(".github/workflows/ci.yml");
        let abs_str = abs.to_str().expect("utf-8 path");
        let resolved = resolve_acceptance_test_path(abs_str);
        assert_eq!(resolved.as_deref(), Some(abs.as_path()));
    }

    #[test]
    fn resolve_absolute_nonexistent_returns_none() {
        let resolved = resolve_acceptance_test_path("/this/path/does/not/exist/anywhere.rs");
        assert!(
            resolved.is_none(),
            "absolute non-existent path returns None"
        );
    }

    #[test]
    fn resolve_relative_nonexistent_returns_none() {
        let resolved = resolve_acceptance_test_path("does/not/exist/anywhere.rs");
        assert!(
            resolved.is_none(),
            "relative non-existent path returns None"
        );
    }

    #[test]
    fn find_repo_root_falls_back_to_cargo_manifest_when_cwd_outside_repo() {
        let saved = std::env::current_dir().expect("cwd");
        let outside = std::env::temp_dir();
        std::env::set_current_dir(&outside).expect("chdir to temp");
        let root = find_repo_root();
        let restore = std::env::set_current_dir(&saved);
        assert!(restore.is_ok(), "cwd restore");
        let root = root.expect("CARGO_MANIFEST_DIR fallback finds repo root from outside cwd");
        assert!(
            root.join(".git").exists(),
            "fallback root must contain .git: {root:?}"
        );
    }

    #[test]
    fn resolve_path_works_from_subdirectory_cwd() {
        let saved = std::env::current_dir().expect("cwd");
        let root = find_repo_root().expect("repo root");
        std::env::set_current_dir(root.join("applications/electron"))
            .expect("chdir to electron subdir");
        let resolved = resolve_acceptance_test_path(".github/workflows/ci.yml");
        let restore = std::env::set_current_dir(&saved);
        assert!(restore.is_ok(), "cwd restore");
        assert!(
            resolved.is_some(),
            "repo-root-relative ATP must resolve from electron subdir cwd"
        );
    }
}
