#![allow(unused_imports)]

use super::super::tools_stories::*;
use protocol::ClientMessage;

#[test]
fn parse_acceptance_test_path_extracts_section_value() {
    let body = "## User job\n\nfoo\n\n## Observation falsifier\n\nbar\n\n## Acceptance test path\n\napplications/electron/tests/foo.spec.ts\n\n---\n\nauthor body";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("applications/electron/tests/foo.spec.ts".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_returns_none_when_section_absent() {
    let body = "## User job\n\nfoo\n\n## Observation falsifier\n\nbar\n\n---\n\nauthor body";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[test]
fn parse_acceptance_test_path_returns_none_when_section_empty() {
    let body = "## User job\n\nfoo\n\n## Acceptance test path\n\n   \n\n---\n\n";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[test]
fn parse_acceptance_test_path_extracts_path_from_bulleted_section() {
    let body = "## Acceptance test path\n\n- Primary: tests/foo.rs (per gate-3 contract)\n- Secondary: existing renderer tests preserve open-state behavior\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("tests/foo.rs".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_strips_trailing_parenthetical_annotation() {
    let body = "## Acceptance test path\n\napplications/bpmux/tests/foo.rs (file new if absent; cross-product test)\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("applications/bpmux/tests/foo.rs".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_handles_label_prefix() {
    let body = "## Acceptance test path\n\nPath: src/foo/bar.ts\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("src/foo/bar.ts".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_accepts_md_under_docs_prefix() {
    let body = "## Acceptance test path\n\ndocs/ops/self-hosted-runner-setup.md\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("docs/ops/self-hosted-runner-setup.md".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_accepts_yml_under_github_workflows_prefix() {
    let body = "## Acceptance test path\n\n.github/workflows/ci.yml\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some(".github/workflows/ci.yml".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_rejects_md_outside_ops_prefix() {
    let body = "## Acceptance test path\n\nsrc/lib.md\n\n";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[test]
fn parse_acceptance_test_path_accepts_source_extension_under_ops_prefix() {
    let body = "## Acceptance test path\n\ndocs/research/foo.rs\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("docs/research/foo.rs".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_accepts_toml_under_docs_prefix() {
    let body = "## Acceptance test path\n\ndocs/config/example.toml\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("docs/config/example.toml".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_returns_none_when_no_path_shaped_line() {
    let body = "## Acceptance test path\n\n- Just prose\n- More prose\n\n";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[test]
fn parse_acceptance_test_path_finds_path_among_prose() {
    let body = "## Acceptance test path\n\nThe acceptance test lives at\ntests/important.rs\n(bullet form available too)\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("tests/important.rs".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_strips_backtick_wrapped_path() {
    let body =
        "## Acceptance test path\n\n`applications/bpmux/src/bridge/tools/get_story_tests.rs`\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("applications/bpmux/src/bridge/tools/get_story_tests.rs".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_strips_asterisk_emphasized_path() {
    let body = "## Acceptance test path\n\n*tests/foo.ts*\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("tests/foo.ts".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_strips_underscore_emphasized_path() {
    let body = "## Acceptance test path\n\n_src/foo/bar.kt_\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("src/foo/bar.kt".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_strips_backticks_inside_bullet_label() {
    let body = "## Acceptance test path\n\n- Primary: `tests/canonical.rs` (load-bearing)\n\n";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("tests/canonical.rs".to_string())
    );
}
