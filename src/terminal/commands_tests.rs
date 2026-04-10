#![allow(clippy::unwrap_used)]

//! Tests for command-layer helpers. Extracted from `commands.rs` to keep
//! that file under the 400-line soft limit — adding #158's
//! `parse_addressee_prefix` + its test module pushed it over the wall.

use super::commands::parse_addressee_prefix;

#[test]
fn plain_text_has_no_addressee() {
    let (a, t) = parse_addressee_prefix("hello world");
    assert_eq!(a, None);
    assert_eq!(t, "hello world");
}

#[test]
fn at_role_prefix_is_extracted() {
    let (a, t) = parse_addressee_prefix("@pm what about X?");
    assert_eq!(a.as_deref(), Some("pm"));
    assert_eq!(t, "what about X?");
}

#[test]
fn at_all_is_valid_addressee() {
    let (a, t) = parse_addressee_prefix("@all heads up everyone");
    assert_eq!(a.as_deref(), Some("all"));
    assert_eq!(t, "heads up everyone");
}

#[test]
fn at_mention_mid_text_is_not_a_prefix() {
    let (a, t) = parse_addressee_prefix("foo @mention bar");
    assert_eq!(a, None);
    assert_eq!(t, "foo @mention bar");
}

#[test]
fn bare_at_is_passthrough() {
    let (a, t) = parse_addressee_prefix("@");
    assert_eq!(a, None);
    assert_eq!(t, "@");
}

#[test]
fn at_role_without_space_is_not_a_prefix() {
    // `@pm` with no space after is treated as the full message —
    // avoids surprise for email-like text or typos.
    let (a, t) = parse_addressee_prefix("@pm");
    assert_eq!(a, None);
    assert_eq!(t, "@pm");
}

#[test]
fn hyphen_in_role_name_allowed() {
    let (a, t) = parse_addressee_prefix("@bpmux-dev hi");
    assert_eq!(a.as_deref(), Some("bpmux-dev"));
    assert_eq!(t, "hi");
}

#[test]
fn multiple_spaces_after_role_collapse() {
    let (a, t) = parse_addressee_prefix("@dev    hello");
    assert_eq!(a.as_deref(), Some("dev"));
    assert_eq!(t, "hello");
}
