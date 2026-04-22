#![allow(clippy::unwrap_used)]

//! Tests for command-layer helpers. Extracted from `commands.rs` to keep
//! that file under the 400-line soft limit — adding #158's
//! `parse_addressee_prefix` + its test module pushed it over the wall.

use super::chat_message::parse_addressee_prefix;

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

// #297 handle_share pure-core extraction — tests for scope-parsing,
// shareable-message-finding, source-scope-resolution, marker-building.
// Functions are pure; tests exercise them without App fixture.

mod share_core {
    use super::super::share::{
        build_share_marker, build_source_scope, find_shareable_message, parse_share_scope,
    };
    use crate::session::{Message, MessageType};
    use chrono::Utc;

    fn chat(from: &str, text: &str, to: Option<Vec<&str>>) -> Message {
        Message {
            from: from.to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            to: to.map(|v| v.into_iter().map(String::from).collect()),
            ..Message::default()
        }
    }

    fn system(text: &str) -> Message {
        Message {
            from: "bpmux".to_string(),
            text: text.to_string(),
            ts: Utc::now(),
            message_type: MessageType::System,
            ..Message::default()
        }
    }

    // parse_share_scope

    #[test]
    fn parse_share_scope_all_sentinel_is_broadcast_none() {
        assert_eq!(
            parse_share_scope(crate::session::message::BROADCAST_ALL),
            Some(None),
            "'all' sentinel must map to broadcast (None recipient list)"
        );
    }

    #[test]
    fn parse_share_scope_single_role_is_one_recipient() {
        assert_eq!(
            parse_share_scope("dev"),
            Some(Some(vec!["dev".to_string()]))
        );
    }

    #[test]
    fn parse_share_scope_comma_list_is_multi_recipient() {
        assert_eq!(
            parse_share_scope("dev,pm,designer"),
            Some(Some(vec![
                "dev".to_string(),
                "pm".to_string(),
                "designer".to_string(),
            ]))
        );
    }

    #[test]
    fn parse_share_scope_trims_whitespace_around_recipients() {
        assert_eq!(
            parse_share_scope("dev ,  pm, designer "),
            Some(Some(vec![
                "dev".to_string(),
                "pm".to_string(),
                "designer".to_string(),
            ]))
        );
    }

    #[test]
    fn parse_share_scope_drops_empty_tokens_in_comma_list() {
        assert_eq!(
            parse_share_scope("dev,,pm"),
            Some(Some(vec!["dev".to_string(), "pm".to_string()]))
        );
    }

    #[test]
    fn parse_share_scope_empty_string_is_invalid() {
        assert_eq!(parse_share_scope(""), None);
    }

    #[test]
    fn parse_share_scope_all_commas_is_invalid() {
        // Collapses to zero recipients after empty-filter → invalid.
        // Caller must show usage; don't silently broadcast.
        assert_eq!(parse_share_scope(",,,"), None);
    }

    // find_shareable_message

    #[test]
    fn find_shareable_message_returns_newest_chat_in_broadcast_view() {
        let older = chat("alice", "earlier", None);
        let newer = chat("bob", "later", None);
        let messages = vec![older, newer.clone()];
        let found = find_shareable_message(&messages, &None, "ryan");
        assert_eq!(found.map(|m| m.text.as_str()), Some("later"));
    }

    #[test]
    fn find_shareable_message_skips_non_chat_messages() {
        // System messages (thread markers, notifications) aren't shareable.
        // Without this filter, /share would propagate internal plumbing.
        let messages = vec![
            chat("alice", "real-msg", None),
            system("thread created: topic"),
        ];
        let found = find_shareable_message(&messages, &None, "ryan");
        assert_eq!(found.map(|m| m.text.as_str()), Some("real-msg"));
    }

    #[test]
    fn find_shareable_message_in_broadcast_view_ignores_directed_messages() {
        // Broadcast view shows only broadcast (to.is_none()). A directed
        // message appearing later doesn't shadow an earlier broadcast.
        let broadcast = chat("alice", "open", None);
        let directed = chat("alice", "direct", Some(vec!["ryan"]));
        let messages = vec![broadcast, directed];
        let found = find_shareable_message(&messages, &None, "ryan");
        assert_eq!(found.map(|m| m.text.as_str()), Some("open"));
    }

    #[test]
    fn find_shareable_message_returns_none_when_only_non_chat_exists() {
        let messages = vec![system("just plumbing")];
        let found = find_shareable_message(&messages, &None, "ryan");
        assert!(found.is_none());
    }

    #[test]
    fn find_shareable_message_returns_none_when_empty() {
        let messages: Vec<Message> = vec![];
        let found = find_shareable_message(&messages, &None, "ryan");
        assert!(found.is_none());
    }

    // build_source_scope

    #[test]
    fn build_source_scope_broadcast_for_none_to() {
        let msg = chat("alice", "hi", None);
        assert_eq!(build_source_scope(&msg, "ryan"), "broadcast");
    }

    #[test]
    fn build_source_scope_resolves_channel_name_for_directed() {
        // Directed message: scope should be the channel name from owner's
        // perspective, not "broadcast". Exact channel-name logic is
        // delegated to channel_name(); here we assert non-"broadcast" is
        // returned + it's some non-empty identifier.
        let msg = chat("alice", "hi", Some(vec!["ryan"]));
        let scope = build_source_scope(&msg, "ryan");
        assert_ne!(scope, "broadcast");
        assert!(!scope.is_empty());
    }

    // build_share_marker

    #[test]
    fn build_share_marker_form1_includes_source_scope() {
        // Form 1: existing message propagated from a source scope. The
        // marker names the origin so the receiving scope can contextualize
        // the propagation.
        let marker = build_share_marker("dev", Some("alice"), "the original text");
        assert_eq!(
            marker,
            "[Shared by dev from alice thread] the original text"
        );
    }

    #[test]
    fn build_share_marker_form2_omits_source_scope() {
        // Form 2: fresh body typed at /share time; no origin thread
        // exists. Marker is shorter — no "from X thread" clause.
        let marker = build_share_marker("dev", None, "typed body");
        assert_eq!(marker, "[Shared by dev] typed body");
    }

    #[test]
    fn build_share_marker_handles_multi_word_actor() {
        // Actors with spaces / special chars preserve through formatting;
        // no escaping needed because marker is display text, not parsing
        // input.
        let marker = build_share_marker("dev-agent", Some("broadcast"), "hello");
        assert_eq!(marker, "[Shared by dev-agent from broadcast thread] hello");
    }
}
