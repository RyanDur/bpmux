#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::section::{derive_iteration_member_section, substitute_member_stories_section};

#[test]
fn derive_member_section_returns_placeholder_when_empty_1462() {
    let section = derive_iteration_member_section(&[]);
    assert_eq!(section, "_(no members yet)_\n");
}

#[test]
fn derive_member_section_renders_single_member_as_bullet_1462() {
    let section = derive_iteration_member_section(&[(1309, "Add ci_get_job_status MCP tool")]);
    assert_eq!(section, "- #1309 — Add ci_get_job_status MCP tool\n");
}

#[test]
fn derive_member_section_renders_multi_member_in_input_order_1462() {
    let members = [
        (1309, "Add ci_get_job_status MCP tool"),
        (1265, "Reorder_story PM-only authority-gate"),
        (1462, "Iteration body auto-derive"),
    ];
    let section = derive_iteration_member_section(&members);
    assert_eq!(
        section,
        "- #1309 — Add ci_get_job_status MCP tool\n\
         - #1265 — Reorder_story PM-only authority-gate\n\
         - #1462 — Iteration body auto-derive\n"
    );
}

#[test]
fn substitute_member_section_replaces_placeholder_with_derived_list_1462() {
    let body = "## Iteration Telescopium\n\
                \n\
                **Created**: 2026-05-19\n\
                **Status**: Open\n\
                \n\
                ## Member stories\n\
                \n\
                <!-- Members carry `iteration-Telescopium.{position}` labels;\n\
                     ordering is by position ascending. -->\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                \n\
                bpmux iteration substrate.\n";
    let derived = "- #1309 — Add ci_get_job_status MCP tool\n\
                   - #1265 — Reorder_story PM-only authority-gate\n";
    let out = substitute_member_stories_section(body, derived);
    assert!(
        out.contains("## Member stories\n"),
        "header preserved: {out}"
    );
    assert!(
        out.contains("- #1309 — Add ci_get_job_status MCP tool\n"),
        "first member appears: {out}"
    );
    assert!(
        out.contains("- #1265 — Reorder_story PM-only authority-gate\n"),
        "second member appears: {out}"
    );
    assert!(
        !out.contains("_(no members yet)_"),
        "placeholder replaced: {out}"
    );
    assert!(out.contains("## Notes\n"), "next H2 preserved: {out}");
    assert!(
        out.contains("bpmux iteration substrate."),
        "downstream Notes content preserved: {out}"
    );
}

#[test]
fn substitute_member_section_with_empty_derived_keeps_placeholder_form_1462() {
    let body = "## Iteration Telescopium\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                bpmux.\n";
    let derived = derive_iteration_member_section(&[]);
    let out = substitute_member_stories_section(body, &derived);
    assert!(
        out.contains("_(no members yet)_"),
        "empty derivation surfaces placeholder: {out}"
    );
    assert!(out.contains("## Notes\n"), "next H2 preserved: {out}");
}

#[test]
fn substitute_member_section_returns_body_unchanged_when_header_missing_1462() {
    let body = "## Iteration No-Members-Section\n\n## Notes\nbpmux.\n";
    let derived = "- #1234 — some title\n";
    let out = substitute_member_stories_section(body, derived);
    assert_eq!(out, body, "no member-stories header → body unchanged");
}

#[test]
fn substitute_member_section_handles_body_without_trailing_h2_1462() {
    let body = "## Iteration Solo\n\n## Member stories\n\n_(no members yet)_\n";
    let derived = "- #42 — Solo member\n";
    let out = substitute_member_stories_section(body, derived);
    assert!(
        out.contains("- #42 — Solo member\n"),
        "derived list inserted when no trailing H2: {out}"
    );
    assert!(
        !out.contains("_(no members yet)_"),
        "placeholder replaced: {out}"
    );
}

#[test]
fn substitute_member_section_idempotent_under_re_derivation_1462() {
    let body = "## Iteration Pegasus\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                end.\n";
    let derived = "- #100 — first\n- #200 — second\n";
    let once = substitute_member_stories_section(body, derived);
    let twice = substitute_member_stories_section(&once, derived);
    assert_eq!(
        once, twice,
        "re-deriving with same input is idempotent (Scenarios 3-4 reactive-on-read)"
    );
}

#[test]
fn substitute_member_section_re_derivation_reflects_membership_mutation_1462() {
    let body = "## Iteration Carina\n\
                \n\
                ## Member stories\n\
                \n\
                _(no members yet)_\n\
                \n\
                ## Notes\n\
                end.\n";
    let before = substitute_member_stories_section(body, "- #1 — a\n- #2 — b\n");
    let after_remove = substitute_member_stories_section(&before, "- #2 — b\n");
    assert!(
        !after_remove.contains("- #1 — a\n"),
        "removed story disappears on re-derivation: {after_remove}"
    );
    assert!(
        after_remove.contains("- #2 — b\n"),
        "remaining member preserved: {after_remove}"
    );
    let after_add = substitute_member_stories_section(&after_remove, "- #2 — b\n- #3 — c\n");
    assert!(
        after_add.contains("- #3 — c\n"),
        "added story appears on re-derivation: {after_add}"
    );
}
