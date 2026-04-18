#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::app::*;
use crate::session::Message;
use crate::session::MessageType;
use crate::terminal::commands::MAX_HISTORY;
use crate::terminal::input::SLASH_COMMANDS;
use chrono::Utc;

fn test_app() -> App {
    App::new("ryan".to_string(), "test-session".to_string(), vec![])
}

fn chat_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        ..Message::default()
    }
}

fn directed_msg(from: &str, text: &str, to: Vec<&str>) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        to: Some(to.into_iter().map(String::from).collect()),
        ..Message::default()
    }
}

fn system_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    }
}

fn activity_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Activity,
        ..Message::default()
    }
}

#[test]
fn submits_message_and_clears_input() {
    let mut app = test_app();
    app.editor.input = "hello world".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.from, "ryan:human");
    assert_eq!(msg.text, "hello world");
    assert!(app.editor.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
}

// ── #268 re-work: thread-tagged submit ──────────────────────
//
// When the viewer is in a thread, typed messages MUST carry the
// thread_id — otherwise the thread-scope filter skips them and the
// user's own input "disappears" from their view (Ryan's
// live-acceptance experience at the feature's first use).

// ── #275: threads section in sidebar ─────────────────────────
//
// Self-participant threads appear in the sidebar with per-thread
// unread counts; clicking switches view. Rows are ordered freshest-
// activity-first so recent conversations surface.

fn mk_thread(app: &mut App, id: &str, name: &str, participants: &[&str]) {
    app.threads.known.insert(
        id.to_string(),
        super::app::Thread {
            id: id.to_string(),
            name: name.to_string(),
            participants: participants.iter().map(|p| (*p).to_string()).collect(),
            last_activity: Some(chrono::Utc::now()),
            unread: 0,
        },
    );
}

#[test]
fn sidebar_lists_only_threads_where_self_is_participant() {
    let mut app = test_app();
    mk_thread(&mut app, "t-in", "in", &["ryan", "dev"]);
    mk_thread(&mut app, "t-out", "out", &["dev", "pm"]);
    let channels = app.sidebar_channels();
    assert!(
        channels.iter().any(|c| c == "thread:t-in"),
        "self-participant thread must appear"
    );
    assert!(
        !channels.iter().any(|c| c == "thread:t-out"),
        "non-participant thread must not appear: {channels:?}"
    );
}

#[test]
fn sidebar_threads_sorted_freshest_first() {
    use chrono::Duration;
    let mut app = test_app();
    let now = chrono::Utc::now();
    app.threads.known.insert(
        "t-old".to_string(),
        super::app::Thread {
            id: "t-old".to_string(),
            name: "old".to_string(),
            participants: vec!["ryan".to_string()],
            last_activity: Some(now - Duration::minutes(30)),
            unread: 0,
        },
    );
    app.threads.known.insert(
        "t-new".to_string(),
        super::app::Thread {
            id: "t-new".to_string(),
            name: "new".to_string(),
            participants: vec!["ryan".to_string()],
            last_activity: Some(now),
            unread: 0,
        },
    );
    let channels = app.sidebar_channels();
    let new_idx = channels.iter().position(|c| c == "thread:t-new").unwrap();
    let old_idx = channels.iter().position(|c| c == "thread:t-old").unwrap();
    assert!(
        new_idx < old_idx,
        "freshest activity should surface first. got: {channels:?}"
    );
}

#[test]
fn sidebar_select_thread_switches_view() {
    let mut app = test_app();
    mk_thread(&mut app, "t-cheese", "cheese", &["ryan", "dev"]);
    let idx = app
        .sidebar_channels()
        .iter()
        .position(|c| c == "thread:t-cheese")
        .expect("thread row");
    app.sidebar.selected = idx;
    app.sidebar_select();
    assert_eq!(app.threads.current.as_deref(), Some("t-cheese"));
}

#[test]
fn sidebar_select_thread_resets_unread() {
    let mut app = test_app();
    mk_thread(&mut app, "t-cheese", "cheese", &["ryan"]);
    app.threads.known.get_mut("t-cheese").unwrap().unread = 3;
    let idx = app
        .sidebar_channels()
        .iter()
        .position(|c| c == "thread:t-cheese")
        .expect("thread row");
    app.sidebar.selected = idx;
    app.sidebar_select();
    assert_eq!(app.threads.known["t-cheese"].unread, 0);
}

// ── #275 rework: thread state rehydrates from session log ──────
//
// Post-restart, App::new replays THREAD_CREATE and THREAD_JOIN
// markers from the incoming messages so threads.known isn't empty
// and the sidebar + thread-scoped filters work immediately — no
// wait for a fresh thread event.

fn system_marker(text: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    }
}

#[test]
fn app_new_rehydrates_thread_create_markers_from_log() {
    let messages = vec![system_marker(
        "__BPMUX_THREAD_CREATE__|cheese-abc|cheese|ryan,dev",
    )];
    let app = App::new("ryan".to_string(), "test".to_string(), messages);
    assert!(
        app.threads.known.contains_key("cheese-abc"),
        "thread must be rehydrated from create marker"
    );
    let t = &app.threads.known["cheese-abc"];
    assert_eq!(t.name, "cheese");
    assert_eq!(t.participants, vec!["ryan".to_string(), "dev".to_string()]);
}

#[test]
fn app_new_rehydrates_thread_join_updates_participants() {
    let messages = vec![
        system_marker("__BPMUX_THREAD_CREATE__|cheese-abc|cheese|ryan"),
        system_marker("__BPMUX_THREAD_JOIN__|cheese|designer"),
    ];
    let app = App::new("ryan".to_string(), "test".to_string(), messages);
    let t = &app.threads.known["cheese-abc"];
    assert!(
        t.participants.iter().any(|p| p == "designer"),
        "join marker must add participant; got: {:?}",
        t.participants
    );
}

#[test]
fn app_new_with_empty_log_has_no_threads() {
    let app = App::new("ryan".to_string(), "test".to_string(), vec![]);
    assert!(app.threads.known.is_empty());
}

#[test]
fn app_new_rehydrated_threads_appear_in_sidebar_for_participant() {
    let messages = vec![system_marker(
        "__BPMUX_THREAD_CREATE__|cheese-abc|cheese|ryan,dev",
    )];
    let app = App::new("ryan".to_string(), "test".to_string(), messages);
    let channels = app.sidebar_channels();
    assert!(
        channels.iter().any(|c| c == "thread:cheese-abc"),
        "rehydrated thread must appear in sidebar"
    );
}

// ── #224 v2: focus-as-addressee auto-tagging ─────────────────
//
// When focus is active on a single role, outgoing submits auto-set
// `addressee` to that role (sticky attention signal). Explicit
// `@role` prefix still wins over focus-sticky (user intent beats
// the ambient state). Multi-role focus doesn't auto-address because
// there's no single target.

#[test]
fn submit_with_single_focus_auto_addresses_focused_role() {
    let mut app = test_app();
    app.focused_roles = Some(vec!["dev".to_string()]);
    app.editor.input = "what do you think".to_string();
    let msg = app.submit_message().expect("message");
    assert_eq!(msg.addressee.as_deref(), Some("dev"));
}

#[test]
fn submit_without_focus_leaves_addressee_none() {
    let mut app = test_app();
    assert!(app.focused_roles.is_none());
    app.editor.input = "everyone".to_string();
    let msg = app.submit_message().expect("message");
    assert!(msg.addressee.is_none());
}

#[test]
fn explicit_at_prefix_wins_over_focus_sticky() {
    // User types `@pm ...` while focused on dev — the explicit
    // prefix beats the ambient focus, matching #158's user-intent
    // priority.
    let mut app = test_app();
    app.focused_roles = Some(vec!["dev".to_string()]);
    app.editor.input = "@pm quick question".to_string();
    let msg = app.submit_message().expect("message");
    assert_eq!(msg.addressee.as_deref(), Some("pm"));
    assert_eq!(msg.text, "quick question");
}

#[test]
fn multi_role_focus_does_not_auto_address() {
    // No unambiguous target when multiple roles are focused — fall
    // back to no addressee rather than guess.
    let mut app = test_app();
    app.focused_roles = Some(vec!["dev".to_string(), "pm".to_string()]);
    app.editor.input = "broadcasting to both".to_string();
    let msg = app.submit_message().expect("message");
    assert!(msg.addressee.is_none());
}

#[test]
fn submit_in_thread_tags_message_with_thread_id() {
    let mut app = test_app();
    app.threads.current = Some("cheese-123".to_string());
    app.editor.input = "what about brie".to_string();
    let msg = app.submit_message().expect("message");
    assert_eq!(
        msg.thread_id.as_deref(),
        Some("cheese-123"),
        "submit in thread must tag the outgoing message"
    );
}

#[test]
fn submit_without_thread_leaves_thread_id_none() {
    let mut app = test_app();
    assert!(app.threads.current.is_none());
    app.editor.input = "broadcast hello".to_string();
    let msg = app.submit_message().expect("message");
    assert!(
        msg.thread_id.is_none(),
        "submit outside a thread must not tag"
    );
}

#[test]
fn submit_in_thread_renders_in_thread_view() {
    // End-to-end regression for "i can't see what i'm saying": with
    // thread_id tagged on the outgoing message, the filter pipeline
    // must include it in the thread's rendered index.
    let mut app = test_app();
    app.threads.current = Some("cheese-123".to_string());
    app.editor.input = "mine should render".to_string();
    app.submit_message();
    app.rebuild_filtered_if_dirty();
    assert_eq!(
        app.msg.filtered_indices.len(),
        1,
        "own thread message must appear in the thread view"
    );
}

#[test]
fn ignores_empty_input() {
    let mut app = test_app();
    app.editor.input = "   ".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
}

#[test]
fn quit_command_sets_flag() {
    let mut app = test_app();
    app.editor.input = "/quit".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.lifecycle.should_quit);
}

#[test]
fn scroll_up_increases_offset() {
    let mut app = test_app();
    app.scroll.max = 100;
    app.scroll_up(5);
    assert_eq!(app.scroll.offset, 5);
    app.scroll_up(3);
    assert_eq!(app.scroll.offset, 8);
}

#[test]
fn scroll_up_clamps_to_max_scroll() {
    let mut app = test_app();
    app.scroll.max = 10;
    app.scroll_up(15);
    assert_eq!(app.scroll.offset, 10);
}

#[test]
fn scroll_down_decreases_offset_to_zero() {
    let mut app = test_app();
    app.scroll.offset = 10;
    app.scroll_down(3);
    assert_eq!(app.scroll.offset, 7);
    app.scroll_down(10);
    assert_eq!(app.scroll.offset, 0);
}

#[test]
fn receiving_message_when_scrolled_up_increments_unread_count() {
    let mut app = test_app();
    app.scroll.offset = 5;
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll.unread_count, 1);
    app.receive_message(chat_msg("bot:ws", "world"));
    assert_eq!(app.scroll.unread_count, 2);
}

#[test]
fn scrolling_to_bottom_resets_unread_count() {
    let mut app = test_app();
    app.scroll.offset = 3;
    app.scroll.unread_count = 5;
    app.scroll_down(3);
    assert_eq!(app.scroll.offset, 0);
    assert_eq!(app.scroll.unread_count, 0);
}

#[test]
fn receiving_message_at_bottom_does_not_increment_unread_count() {
    let mut app = test_app();
    assert_eq!(app.scroll.offset, 0);
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll.unread_count, 0);
}

#[test]
fn incoming_message_stays_at_bottom_when_at_bottom() {
    let mut app = test_app();
    assert_eq!(app.scroll.offset, 0);
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll.offset, 0);
    assert_eq!(app.msg.messages.len(), 1);
}

#[test]
fn up_arrow_recalls_last_sent_message() {
    let mut app = test_app();
    app.editor.input = "hello".to_string();
    app.submit_message();
    app.editor.input = "world".to_string();
    app.submit_message();

    app.history_up();
    assert_eq!(app.editor.input, "world");
    assert_eq!(app.editor.history_index, Some(1));

    app.history_up();
    assert_eq!(app.editor.input, "hello");
    assert_eq!(app.editor.history_index, Some(0));
}

#[test]
fn down_arrow_returns_to_empty_after_recalling() {
    let mut app = test_app();
    app.editor.input = "hello".to_string();
    app.submit_message();

    app.history_up();
    assert_eq!(app.editor.input, "hello");

    app.history_down();
    assert!(app.editor.input.is_empty());
    assert_eq!(app.editor.history_index, None);
}

#[test]
fn history_caps_at_50_entries() {
    let mut app = test_app();
    for i in 0..60 {
        app.editor.input = format!("msg {i}");
        app.submit_message();
    }
    assert_eq!(app.editor.input_history.len(), MAX_HISTORY);
    assert_eq!(app.editor.input_history[0], "msg 10");
    assert_eq!(app.editor.input_history[49], "msg 59");
}

#[test]
fn who_command_lists_peer_name_and_returns_none() {
    let mut app = test_app();
    app.editor.input = "/who".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.editor.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
    assert_eq!(app.msg.messages[0].from, "bpmux");
    assert!(app.msg.messages[0].text.contains("ryan (you)"));
}

#[test]
fn who_command_includes_peer_activity() {
    let mut app = test_app();
    app.update_activity("dev".to_string(), "working".to_string());
    app.editor.input = "/who".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("dev [working]"));
}

#[test]
fn unrecognized_command_adds_error_message_without_broadcast() {
    let mut app = test_app();
    app.editor.input = "/foo".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.editor.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
    assert_eq!(app.msg.messages[0].from, "bpmux");
    assert!(app.msg.messages[0].text.contains("Unknown command"));
    assert!(app.msg.messages[0].text.contains("/foo"));
    assert!(app.msg.messages[0].text.contains("?"));
}

#[test]
fn incoming_message_preserves_viewport_when_scrolled_up() {
    let mut app = test_app();
    app.scroll.offset = 5;
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll.offset, 6);
    assert_eq!(app.msg.messages.len(), 1);
}

#[test]
fn update_activity_adds_entry() {
    let mut app = test_app();
    app.update_activity("dev".to_string(), "working".to_string());
    assert!(app.peers.activity.contains_key("dev"));
    let (status, _) = &app.peers.activity["dev"];
    assert_eq!(status, "working");
}

#[test]
fn activity_message_does_not_appear_in_messages() {
    let mut app = test_app();
    app.receive_message(activity_msg("dev:claude-code", "working"));
    assert!(app.msg.messages.is_empty());
    assert!(app.peers.activity.contains_key("dev"));
}

#[test]
fn peer_activity_not_rebuilt_from_log() {
    let messages = vec![
        system_msg("bpmux", "dev:ws joined"),
        activity_msg("dev:ws", "busy"),
        system_msg("bpmux", "pm:ws joined"),
    ];
    let app = App::new("ryan".to_string(), "test-session".to_string(), messages);
    assert!(
        app.peers.activity.is_empty(),
        "peer_activity should be empty on startup — only live connections populate it"
    );
}

#[test]
fn live_join_uses_short_name() {
    let mut app = test_app();
    app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
    app.receive_message(system_msg("bpmux", "dev:ws joined"));
    assert_eq!(
        app.peers.activity.len(),
        1,
        "same role via different transports should dedupe"
    );
    assert!(app.peers.activity.contains_key("dev"));
}

#[test]
fn live_leave_removes_by_short_name() {
    let mut app = test_app();
    app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
    assert!(app.peers.activity.contains_key("dev"));
    app.receive_message(system_msg("bpmux", "dev:ws left"));
    assert!(
        !app.peers.activity.contains_key("dev"),
        "leave from any transport should remove"
    );
}

#[test]
fn debug_toggles_show_system_messages() {
    let mut app = test_app();
    assert!(!app.display.show_system_messages);

    app.editor.input = "/debug".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.display.show_system_messages);
    assert!(app.editor.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
    assert!(app.msg.messages[0].text.contains("Debug mode on"));
    assert!(app.msg.messages[0].text.contains("visible"));

    app.editor.input = "/debug".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(!app.display.show_system_messages);
    assert_eq!(app.msg.messages.len(), 2);
    assert!(app.msg.messages[1].text.contains("Debug mode off"));
    assert!(app.msg.messages[1].text.contains("hidden"));
}

#[test]
fn to_single_recipient_returns_directed_message() {
    let mut app = test_app();
    app.editor.input = "/to dev hello there".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.from, "ryan:human");
    assert_eq!(msg.text, "hello there");
    assert_eq!(msg.to, Some(vec!["dev".to_string()]));
}

#[test]
fn to_csv_recipients_returns_directed_message() {
    let mut app = test_app();
    app.editor.input = "/to dev,designer check this".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.text, "check this");
    assert_eq!(
        msg.to,
        Some(vec!["dev".to_string(), "designer".to_string()])
    );
}

#[test]
fn to_bare_shows_usage() {
    let mut app = test_app();
    app.editor.input = "/to".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("/"));
}

#[test]
fn to_recipient_no_body_shows_usage() {
    let mut app = test_app();
    app.editor.input = "/to dev".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("/"));
}

#[test]
fn tab_completes_participant_name_after_to() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.update_activity("designer:claude-code".to_string(), "idle".to_string());
    app.editor.input = "/to de".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    assert!(app.tab_complete());
    // Should complete to first alphabetical match
    assert!(
        app.editor.input.starts_with("/to de"),
        "should complete a 'de' prefix, got: {}",
        app.editor.input
    );
    assert!(
        app.editor.input.ends_with(' '),
        "should add trailing space for body"
    );
}

#[test]
fn tab_completes_csv_second_recipient() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.update_activity("pm:claude-code".to_string(), "idle".to_string());
    app.editor.input = "/to dev,p".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    assert!(app.tab_complete());
    assert_eq!(app.editor.input, "/to dev,pm ");
}

#[test]
fn tab_does_nothing_without_to_prefix() {
    let mut app = test_app();
    app.editor.input = "hello".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    assert!(!app.tab_complete());
    assert_eq!(app.editor.input, "hello");
}

#[test]
fn tab_does_nothing_in_body_portion() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.editor.input = "/to dev hel".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    assert!(!app.tab_complete());
}

#[test]
fn dm_increments_unread_and_sets_flash() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hey ryan", vec!["ryan"]));
    assert_eq!(app.peers.dm_unread_from.len(), 1);
    assert!(app.peers.dm_flash_until.is_some());
}

#[test]
fn broadcast_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(chat_msg("pm:claude-code", "hello all"));
    assert_eq!(app.peers.dm_unread_from.len(), 0);
    assert!(app.peers.dm_flash_until.is_none());
}

#[test]
fn to_all_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hello", vec!["all"]));
    assert_eq!(app.peers.dm_unread_from.len(), 0);
}

#[test]
fn dm_to_other_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hey dev", vec!["dev"]));
    assert_eq!(app.peers.dm_unread_from.len(), 0);
}

#[test]
fn submit_clears_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hey", vec!["ryan"]));
    assert_eq!(app.peers.dm_unread_from.len(), 1);
    app.editor.input = "got it".to_string();
    app.submit_message();
    assert_eq!(app.peers.dm_unread_from.len(), 0);
    assert!(app.peers.dm_flash_until.is_none());
}

#[test]
fn broadcast_message_adds_channel() {
    let mut app = test_app();
    assert_eq!(app.channels.active.len(), 0);
    app.receive_message(chat_msg("dev:claude-code", "hello"));
    assert_eq!(app.channels.active.len(), 1);
    assert!(app.channels.active.contains("broadcast"));
}

#[test]
fn directed_message_adds_new_channel() {
    let mut app = test_app();
    app.receive_message(chat_msg("dev:claude-code", "broadcast"));
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    assert_eq!(app.channels.active.len(), 2);
}

#[test]
fn same_channel_does_not_duplicate() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    app.receive_message(directed_msg("dev:claude-code", "also hey", vec!["ryan"]));
    assert_eq!(app.channels.active.len(), 1);
    assert!(app.channels.active.contains("dev"));
}

#[test]
fn different_senders_create_different_channels() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    app.receive_message(directed_msg("pm:claude-code", "also hey", vec!["ryan"]));
    assert_eq!(app.channels.active.len(), 2);
    assert!(app.channels.active.contains("dev"));
    assert!(app.channels.active.contains("pm"));
}

#[test]
fn channel_name_broadcast() {
    let msg = chat_msg("dev:claude-code", "hi");
    assert_eq!(channel_name(&msg, "ryan"), "broadcast");
}

#[test]
fn channel_name_to_all_is_broadcast() {
    let msg = directed_msg("dev:claude-code", "hi", vec!["all"]);
    assert_eq!(channel_name(&msg, "ryan"), "broadcast");
}

#[test]
fn channel_name_dm_excludes_owner() {
    // dev sends to ryan — owner (ryan) is excluded, channel is just "dev"
    let msg = directed_msg("dev:claude-code", "hey", vec!["ryan"]);
    assert_eq!(channel_name(&msg, "ryan"), "dev");
}

#[test]
fn channel_name_group_excludes_owner() {
    // pm sends to dev and ryan — owner excluded, channel is "dev+pm"
    let msg = directed_msg("pm:claude-code", "hey", vec!["dev", "ryan"]);
    assert_eq!(channel_name(&msg, "ryan"), "dev+pm");
}

#[test]
fn channel_name_owner_sending_matches_channel() {
    // ryan sends to designer+dev+pm — same channel as when they talk
    let agent_msg = directed_msg("dev:claude-code", "hey", vec!["designer", "pm"]);
    let owner_msg = directed_msg("ryan:human", "hey", vec!["designer", "dev", "pm"]);
    assert_eq!(
        channel_name(&agent_msg, "ryan"),
        channel_name(&owner_msg, "ryan"),
        "owner sending into a channel should produce the same channel name"
    );
}

#[test]
fn activity_messages_do_not_create_channels() {
    let mut app = test_app();
    app.receive_message(activity_msg("dev:claude-code", "busy"));
    assert_eq!(app.channels.active.len(), 0);
}

// ── receive_message coverage gates (#261 split safety net) ──
//
// Behaviors below were already wired into receive_message but lacked
// dedicated tests. Codifying them here BEFORE the per-MessageType
// reducer split per designer's pickup-gate concern: "555 tests pass"
// is not sufficient proof that each branch's side effects survive
// extraction. These tests pin down each reducer's contract.

#[test]
fn chat_message_with_status_prefix_updates_activity() {
    let mut app = test_app();
    app.receive_message(chat_msg("dev:claude-code", "/status hacking on tests"));
    let (status, _) = app
        .peers
        .activity
        .get("dev")
        .expect("dev activity should be set");
    assert_eq!(status, "hacking on tests");
}

#[test]
fn insight_message_records_in_log_like_chat() {
    let mut app = test_app();
    let msg = Message {
        from: "dev:claude-code".to_string(),
        text: "reasoning trace".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Insight,
        ..Message::default()
    };
    app.receive_message(msg);
    assert_eq!(app.msg.messages.len(), 1);
    assert_eq!(app.msg.messages[0].text, "reasoning trace");
}

#[test]
fn nudge_message_records_in_log_like_chat() {
    let mut app = test_app();
    let msg = Message {
        from: "bpmux".to_string(),
        text: "📝 nudge".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Nudge,
        ..Message::default()
    };
    app.receive_message(msg);
    assert_eq!(app.msg.messages.len(), 1);
}

#[test]
fn directed_chat_message_marks_dm_unread_from_sender() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hi", vec!["ryan"]));
    assert!(app.peers.dm_unread_from.contains(&"dev".to_string()));
    assert!(app.peers.dm_flash_until.is_some());
}

#[test]
fn broadcast_to_all_does_not_mark_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hi", vec!["all"]));
    assert!(app.peers.dm_unread_from.is_empty());
    assert!(app.peers.dm_flash_until.is_none());
}

#[test]
fn popup_matches_filters_by_typed_prefix() {
    let mut app = test_app();
    app.editor.input = "/wh".to_string();
    let m = app.popup_matches();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, "/who");
}

#[test]
fn popup_matches_returns_all_on_bare_slash() {
    let mut app = test_app();
    app.editor.input = "/".to_string();
    assert_eq!(app.popup_matches().len(), SLASH_COMMANDS.len());
}

#[test]
fn popup_matches_empty_after_space() {
    let mut app = test_app();
    app.editor.input = "/to dev".to_string();
    assert!(app.popup_matches().is_empty());
}

#[test]
fn popup_navigation_wraps_around() {
    let mut app = test_app();
    app.editor.input = "/".to_string();
    let count = app.popup_matches().len();

    // Down from None starts at 0
    app.popup_down();
    assert_eq!(app.editor.popup_selected, Some(0));

    // Up from 0 wraps to last
    app.popup_up();
    assert_eq!(app.editor.popup_selected, Some(count - 1));

    // Down from last wraps to 0
    app.popup_down();
    assert_eq!(app.editor.popup_selected, Some(0));
}

#[test]
fn popup_accept_fills_input() {
    let mut app = test_app();
    app.editor.input = "/t".to_string();
    app.popup_down(); // select /to
    assert!(app.popup_accept());
    assert_eq!(app.editor.input, "/to ");
    assert_eq!(app.editor.cursor_pos, 4);
    assert!(app.editor.popup_selected.is_none());
}

#[test]
fn popup_selected_resets_on_input_change() {
    let mut app = test_app();
    app.editor.input = "/".to_string();
    app.editor.cursor_pos = 1;
    app.popup_down();
    assert!(app.editor.popup_selected.is_some());
    app.insert_char('t');
    assert!(app.editor.popup_selected.is_none());
}

// Sidebar tests

#[test]
fn sidebar_channels_starts_with_project_node() {
    let app = test_app();
    let channels = app.sidebar_channels();
    assert_eq!(channels[0], "test-session");
}

#[test]
fn sidebar_channels_includes_connected_peers() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.update_activity("dev".to_string(), "available".to_string());
    let channels = app.sidebar_channels();
    assert!(channels.contains(&"broadcast".to_string()));
    assert!(channels.contains(&"dev".to_string()));
}

#[test]
fn sidebar_toggle_three_state_cycle() {
    let mut app = test_app();
    // Closed -> open + focused
    assert!(!app.sidebar.open);
    app.toggle_sidebar();
    assert!(app.sidebar.open);
    assert!(app.sidebar.focused);
    // Open + focused -> closed
    app.toggle_sidebar();
    assert!(!app.sidebar.open);
    assert!(!app.sidebar.focused);
    // Reopen, then unfocus (simulating channel select)
    app.toggle_sidebar();
    app.sidebar.focused = false; // simulate select unfocusing
    // Open + unfocused -> refocused
    app.toggle_sidebar();
    assert!(app.sidebar.open);
    assert!(app.sidebar.focused);
}

#[test]
fn sidebar_select_all_clears_view_and_target() {
    let mut app = test_app();
    app.channels.current_view = Some("broadcast".to_string());
    app.channels.send_target = Some("broadcast".to_string());
    app.sidebar.selected = 0; // project node is always first
    app.sidebar_select();
    assert!(app.channels.current_view.is_none());
    assert!(app.channels.send_target.is_none());
}

#[test]
fn sidebar_select_channel_sets_view_and_target() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.sidebar.selected = 1; // "broadcast" (sorted after project node)
    app.sidebar_select();
    assert_eq!(app.channels.current_view, Some("broadcast".to_string()));
    assert_eq!(app.channels.send_target, Some("broadcast".to_string()));
}

#[test]
fn sidebar_select_clears_unread() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.channels.unread.insert("broadcast".to_string(), 5);
    app.sidebar.selected = 1;
    app.sidebar_select();
    assert_eq!(app.channels.unread.get("broadcast"), None);
}

#[test]
fn clicking_channel_sets_current_view_and_resets_filter_and_scroll() {
    // Regression guard for the sidebar-click path that replaces the
    // deleted `/view` slash command. Asserts every effect the shared
    // switch_view_to helper must produce so any drift is caught here.
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.msg.filter_dirty = false;
    app.scroll.offset = 7;

    app.sidebar.selected = 1; // broadcast sits at index 1
    app.sidebar_select();

    assert_eq!(
        app.channels.current_view,
        Some("broadcast".to_string()),
        "click should set current_view to the clicked channel"
    );
    assert!(
        app.msg.filter_dirty,
        "click should mark message filters dirty"
    );
    assert_eq!(app.scroll.offset, 0, "click should reset scroll offset");
}

// ── #224: clickable peer focus ──────────────────────────────────
//
// Direct-peer rows have two click targets: the name toggles focus;
// the trailing chevron switches view. Keyboard Enter on a peer row
// maps to the name-click path (focus toggle). Non-peer rows (self,
// broadcast, groups, system headers) keep the view-switch behavior
// regardless of click column.

fn sidebar_with_peers(peers: &[&str]) -> App {
    let mut app = test_app();
    for p in peers {
        app.update_activity((*p).to_string(), "idle".to_string());
    }
    app
}

fn peer_row_index(app: &App, peer: &str) -> usize {
    app.sidebar_channels()
        .iter()
        .position(|c| c == peer)
        .expect("peer row")
}

#[test]
fn clicking_peer_name_adds_it_to_focused_roles() {
    let mut app = sidebar_with_peers(&["dev"]);
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select_at(false); // click on name, not chevron
    assert_eq!(app.focused_roles.as_deref(), Some(&["dev".to_string()][..]));
}

#[test]
fn clicking_an_already_focused_peer_removes_it() {
    let mut app = sidebar_with_peers(&["dev"]);
    let idx = peer_row_index(&app, "dev");
    app.sidebar.selected = idx;
    app.sidebar_select_at(false);
    app.sidebar.selected = idx;
    app.sidebar_select_at(false);
    assert!(app.focused_roles.is_none(), "empty focus collapses to None");
}

#[test]
fn clicking_multiple_peer_names_accumulates_focus() {
    let mut app = sidebar_with_peers(&["dev", "pm"]);
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select_at(false);
    app.sidebar.selected = peer_row_index(&app, "pm");
    app.sidebar_select_at(false);
    let focus = app.focused_roles.as_deref().expect("focus set");
    assert!(focus.contains(&"dev".to_string()));
    assert!(focus.contains(&"pm".to_string()));
    assert_eq!(focus.len(), 2);
}

#[test]
fn clicking_peer_name_does_not_switch_view_or_send_target() {
    let mut app = sidebar_with_peers(&["dev"]);
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select_at(false);
    assert!(app.channels.current_view.is_none());
    assert!(app.channels.send_target.is_none());
}

#[test]
fn clicking_peer_regardless_of_column_toggles_focus_post_v2() {
    // #224 v2: chevron is gone; peer rows have a single click-zone
    // (the whole row) that toggles focus. The `on_chevron` bool
    // is now ignored — both mouse-click variants route through
    // the same focus-toggle path.
    let mut app = sidebar_with_peers(&["dev"]);
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select_at(true); // "on_chevron" ignored post-v2
    assert_eq!(app.focused_roles.as_deref(), Some(&["dev".to_string()][..]));
    assert!(
        app.channels.current_view.is_none(),
        "v2 peer-click no longer switches view"
    );
}

#[test]
fn keyboard_enter_on_peer_toggles_focus_not_view() {
    // sidebar_select (called by Enter) maps to the name-click path
    // per designer's spec — primary action is focus toggle.
    let mut app = sidebar_with_peers(&["dev"]);
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select();
    assert_eq!(app.focused_roles.as_deref(), Some(&["dev".to_string()][..]));
    assert!(app.channels.current_view.is_none());
}

#[test]
fn clicking_peer_matches_typed_focus_command_equivalence() {
    let mut clicked = sidebar_with_peers(&["dev"]);
    clicked.sidebar.selected = peer_row_index(&clicked, "dev");
    clicked.sidebar_select_at(false);

    let mut typed = sidebar_with_peers(&["dev"]);
    typed.editor.input = "/focus dev".to_string();
    typed.submit_message();

    assert_eq!(clicked.focused_roles, typed.focused_roles);
}

#[test]
fn clicking_broadcast_name_still_switches_view_not_focus() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.sidebar.selected = peer_row_index(&app, "broadcast");
    app.sidebar_select_at(false);
    assert_eq!(app.channels.current_view.as_deref(), Some("broadcast"));
    assert!(app.focused_roles.is_none());
}

#[test]
fn clicking_group_channel_still_switches_view_not_focus() {
    let mut app = sidebar_with_peers(&["dev", "pm"]);
    app.channels.active.insert("dev+pm".to_string());
    app.sidebar.selected = peer_row_index(&app, "dev+pm");
    app.sidebar_select_at(false);
    assert_eq!(app.channels.current_view.as_deref(), Some("dev+pm"));
    assert!(app.focused_roles.is_none());
}

#[test]
fn clicking_peer_name_marks_filter_dirty() {
    let mut app = sidebar_with_peers(&["dev"]);
    app.msg.filter_dirty = false;
    app.sidebar.selected = peer_row_index(&app, "dev");
    app.sidebar_select_at(false);
    assert!(app.msg.filter_dirty);
}

#[test]
fn sidebar_navigation_wraps() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    // channels: ["test-session", "broadcast"]
    app.sidebar.selected = 0;
    app.sidebar_up(); // wraps to last
    assert_eq!(app.sidebar.selected, 1);
    app.sidebar_down(); // wraps to first
    assert_eq!(app.sidebar.selected, 0);
}

#[test]
fn sidebar_close_unfocuses_but_stays_open() {
    let mut app = test_app();
    app.toggle_sidebar();
    assert!(app.sidebar.open);
    assert!(app.sidebar.focused);
    app.sidebar_close(); // Esc -- unfocuses but keeps sidebar open
    assert!(app.sidebar.open);
    assert!(!app.sidebar.focused);
}

#[test]
fn sidebar_collapse_hides_channels() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    assert!(app.sidebar.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 2); // project + broadcast
    app.sidebar_collapse();
    assert!(!app.sidebar.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 1); // project only
    assert_eq!(app.sidebar.selected, 0); // snapped to project node
}

#[test]
fn sidebar_expand_shows_channels() {
    let mut app = test_app();
    app.channels.active.insert("broadcast".to_string());
    app.sidebar.project_expanded = false;
    assert_eq!(app.sidebar_channels().len(), 1);
    app.sidebar_expand();
    assert!(app.sidebar.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 2);
}

#[test]
fn sidebar_expanded_by_default() {
    let app = test_app();
    assert!(app.sidebar.project_expanded);
}

#[test]
fn send_target_directs_outgoing_messages() {
    let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
    app.channels.send_target = Some("dev\u{2194}human".to_string());
    app.editor.input = "hello dev".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.to, Some(vec!["dev".to_string()]));
}

#[test]
fn no_send_target_broadcasts() {
    let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
    app.editor.input = "hello everyone".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message().unwrap();
    assert!(msg.to.is_none());
}

#[test]
fn channel_unread_increments_on_receive() {
    let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
    app.channels.current_view = Some("broadcast".to_string());
    app.receive_message(directed_msg("dev:claude-code", "dm message", vec!["human"]));
    assert_eq!(*app.channels.unread.get("dev").unwrap_or(&0), 1);
}

#[test]
fn roadmap_handler_adds_system_message() {
    // handle_roadmap is still invoked by the event loop when the
    // bridge emits VIEW_ROADMAP_MARKER (#226 agent-triggered path).
    // Call it directly instead of via the removed /roadmap slash.
    let mut app = test_app();
    app.handle_roadmap();
    let last = app.msg.messages.last().expect("should have a message");
    assert_eq!(last.from, "bpmux");
    assert!(last.text.contains("bpmux Roadmap") || last.text.contains("Roadmap unavailable"));
}

#[test]
fn velocity_handler_opens_project_panel() {
    // handle_velocity is invoked by the event loop when the bridge
    // emits VIEW_VELOCITY_MARKER (#210 agent-triggered path). The
    // TUI side-effect is a mode flip to Project — velocity already
    // renders inside that panel.
    let mut app = test_app();
    assert!(!matches!(app.input_mode, super::app::InputMode::Project));
    app.handle_velocity();
    assert!(matches!(app.input_mode, super::app::InputMode::Project));
}

#[test]
fn velocity_handler_does_not_clear_input() {
    // Agent-triggered path fires while Ryan may be typing. Wiping
    // his in-progress input would be a surprise, not a feature.
    let mut app = test_app();
    app.editor.input = "typing something".to_string();
    app.handle_velocity();
    assert_eq!(app.editor.input, "typing something");
}

#[test]
fn toggle_expands_long_system_message() {
    let mut app = test_app();
    // Add a long message from "bpmux" (like roadmap output)
    let long_text = "x".repeat(1000); // ~13 wrapped lines at width 78
    app.msg.messages.push(Message {
        from: "bpmux".into(),
        text: long_text,
        ts: chrono::Utc::now(),
        ..Message::default()
    });
    let key = app.msg.messages.last().unwrap().ts.to_rfc3339();
    assert!(
        !app.msg.expanded_messages.contains_key(&key),
        "should not be expanded initially"
    );
    app.toggle_last_truncated();
    assert!(
        app.msg.expanded_messages.contains_key(&key),
        "should be expanded after toggle"
    );
    app.toggle_last_truncated();
    assert!(
        !app.msg.expanded_messages.contains_key(&key),
        "should collapse after second toggle"
    );
}

// Channel cleanup tests

#[test]
fn peer_leave_removes_phantom_channels() {
    let mut app = test_app();
    // Simulate agent connecting and sending a message
    app.receive_message(Message {
        from: "agent-14c3:claude-code".to_string(),
        text: "agent-14c3 joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    });
    // View the channel so the message is "read" (no unread count)
    app.channels.current_view = Some("agent-14c3".to_string());
    app.receive_message(directed_msg(
        "agent-14c3:claude-code",
        "hello",
        vec!["ryan"],
    ));
    assert!(
        app.channels.active.contains("agent-14c3"),
        "channel should exist while agent is connected"
    );

    // Agent leaves — channel should be removed since no unread messages
    app.receive_message(Message {
        from: "bpmux".to_string(),
        text: "agent-14c3 left".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    });
    assert!(
        !app.channels.active.contains("agent-14c3"),
        "read channel should be removed when agent disconnects"
    );
}

#[test]
fn peer_leave_keeps_channels_with_unread() {
    let mut app = test_app();
    // Agent connects, sends DM, then leaves
    app.receive_message(Message {
        from: "dev:claude-code".to_string(),
        text: "dev joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    });
    app.channels.current_view = Some("broadcast".to_string());
    app.receive_message(directed_msg(
        "dev:claude-code",
        "unread message",
        vec!["ryan"],
    ));
    assert!(app.channels.unread.get("dev").is_some_and(|&n| n > 0));

    // Agent leaves
    app.receive_message(Message {
        from: "bpmux".to_string(),
        text: "dev left".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    });
    assert!(
        app.channels.active.contains("dev"),
        "channel with unread messages should be preserved"
    );
}

// P2P command tests

#[test]
fn session_command_sets_p2p_command() {
    let mut app = test_app();
    app.editor.input = "/session".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message();
    assert!(
        msg.is_none(),
        "session command produces no broadcast message"
    );
    assert!(
        matches!(
            app.lifecycle.p2p_command,
            Some(crate::transport::peer::P2pCommand::CreateSession)
        ),
        "should set P2P command"
    );
}

#[test]
fn join_command_sets_p2p_command_with_args() {
    let mut app = test_app();
    app.editor.input = "/join blue-tiger-42 someOfferCode123".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    match &app.lifecycle.p2p_command {
        Some(crate::transport::peer::P2pCommand::JoinSession {
            passphrase,
            offer_code,
        }) => {
            assert_eq!(passphrase, "blue-tiger-42");
            assert_eq!(offer_code, "someOfferCode123");
        }
        other => panic!("expected JoinSession, got {other:?}"),
    }
}

#[test]
fn join_without_args_shows_usage() {
    let mut app = test_app();
    app.editor.input = "/join".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(
        app.lifecycle.p2p_command.is_none(),
        "no command without args"
    );
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("/"));
}

#[test]
fn accept_command_sets_p2p_command() {
    let mut app = test_app();
    app.editor.input = "/accept someAnswerCode456".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    match &app.lifecycle.p2p_command {
        Some(crate::transport::peer::P2pCommand::AcceptAnswer { answer_code }) => {
            assert_eq!(answer_code, "someAnswerCode456");
        }
        other => panic!("expected AcceptAnswer, got {other:?}"),
    }
}

#[test]
fn accept_without_args_shows_usage() {
    let mut app = test_app();
    app.editor.input = "/accept".to_string();
    app.editor.cursor_pos = app.editor.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.lifecycle.p2p_command.is_none());
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("/"));
}

// ---------- /share command ----------

#[test]
fn share_bare_shows_usage() {
    let mut app = test_app();
    app.editor.input = "/share".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("/share"));
}

#[test]
fn share_form2_sends_fresh_message_with_marker() {
    // Form 2: /share all "text" sends a new message with a
    // [Shared by <sender>] marker.
    let mut app = test_app();
    app.editor.input = "/share all hello team".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.to, None, "all = broadcast");
    assert!(
        msg.text.starts_with("[Shared by ryan]"),
        "expected Shared-by marker, got: {}",
        msg.text
    );
    assert!(msg.text.contains("hello team"));
    assert!(msg.addressee.is_none(), "addressee not inherited");
    assert!(!msg.explicit_privacy);
}

#[test]
fn share_form2_with_scoped_target() {
    let mut app = test_app();
    app.editor.input = "/share dev,designer coordinating on story X".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(
        msg.to,
        Some(vec!["dev".to_string(), "designer".to_string()])
    );
    assert!(msg.text.contains("coordinating on story X"));
}

#[test]
fn share_form1_propagates_most_recent_from_view() {
    // Form 1: /share all with no body propagates the most recent message
    // in the sender's current view.
    let mut app = test_app();
    app.msg
        .messages
        .push(chat_msg("pm:claude-code", "old message"));
    app.msg
        .messages
        .push(chat_msg("pm:claude-code", "the decision to share"));
    app.channels.current_view = None; // broadcast view
    app.editor.input = "/share dev".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    // Marker format: [Shared by ryan from broadcast thread] <original>
    assert!(
        msg.text
            .starts_with("[Shared by ryan from broadcast thread]"),
        "marker wrong: {}",
        msg.text
    );
    assert!(msg.text.contains("the decision to share"));
    assert!(!msg.text.contains("old message"), "only most recent");
}

#[test]
fn share_form1_refuses_to_propagate_private_to_message() {
    // A /to-scoped message has explicit_privacy=true. /share must refuse
    // to propagate it, per the privacy guarantee.
    let mut app = test_app();
    let mut private = chat_msg("pm:claude-code", "private thing");
    private.to = Some(vec!["ryan".to_string()]);
    private.explicit_privacy = true;
    app.msg.messages.push(private);
    app.channels.current_view = Some("pm".to_string());
    app.editor.input = "/share all".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none(), "private message not propagated");
    let last = app.msg.messages.last().unwrap();
    assert!(
        last.text.contains("Cannot share a private message"),
        "expected privacy refusal, got: {}",
        last.text
    );
}

#[test]
fn share_form1_with_no_recent_message_shows_error() {
    // No messages in the current view — /share form 1 can't pick a target.
    let mut app = test_app();
    app.channels.current_view = None;
    app.editor.input = "/share all".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("No recent message"));
}

#[test]
fn share_does_not_inherit_addressee() {
    // Even if the source message had addressee set, the propagated copy
    // must NOT inherit it. Propagation is awareness, not action.
    let mut app = test_app();
    let mut directed = chat_msg("pm:claude-code", "pm-directed decision");
    directed.addressee = Some("dev".to_string());
    app.msg.messages.push(directed);
    app.channels.current_view = None;
    app.editor.input = "/share designer".to_string();
    let msg = app.submit_message().unwrap();
    assert!(
        msg.addressee.is_none(),
        "addressee must NOT be inherited on propagation, got {:?}",
        msg.addressee
    );
}

#[test]
fn share_source_scope_reflects_channel_name() {
    // channel_name derives the view from (from, to) — it includes the
    // sender and excludes the owner. For a dev-sent message to
    // ["pm","designer"] in ryan's view, channel_name returns
    // "designer+dev+pm" (sorted, excluding "ryan").
    let mut app = test_app();
    let mut scoped = chat_msg("dev:claude-code", "shipping feature X");
    scoped.to = Some(vec!["pm".to_string(), "designer".to_string()]);
    // Not explicit_privacy — this came from a channel view, not /to
    app.msg.messages.push(scoped);
    app.channels.current_view = Some("designer+dev+pm".to_string());
    app.editor.input = "/share all".to_string();
    let msg = app.submit_message().unwrap();
    // Source scope in the marker should be the channel_name of the source
    assert!(
        msg.text.contains("from designer+dev+pm thread]"),
        "expected designer+dev+pm source scope, got: {}",
        msg.text
    );
}

// ---------- Classifier view filter ----------

#[test]
fn all_command_toggles_filter_state() {
    let mut app = test_app();
    assert!(
        !app.display.show_all_classes,
        "filter default is ON (show_all_classes false)"
    );
    app.editor.input = "/all".to_string();
    app.submit_message();
    assert!(
        app.display.show_all_classes,
        "first /all toggles to showing everything"
    );
    app.editor.input = "/all".to_string();
    app.submit_message();
    assert!(
        !app.display.show_all_classes,
        "second /all flips back to filtered"
    );
}

#[test]
fn all_command_does_not_post_to_pane() {
    // Designer flagged 2026-04-10 that the toggle confirmation message
    // was "noise about a noise filter" — the status bar already shows
    // "N hidden" / "showing all", so the chat-pane line was redundant.
    // The /all command now flips state silently. This test guards
    // against regressing the redundant feedback.
    let mut app = test_app();
    let before = app.msg.messages.len();
    app.editor.input = "/all".to_string();
    app.submit_message();
    assert_eq!(
        app.msg.messages.len(),
        before,
        "/all should flip state silently (status bar carries the feedback)"
    );
    assert!(app.display.show_all_classes, "state still flipped");
}

#[test]
fn ui_state_persists_show_all_classes() {
    use crate::terminal::ui_state::UiState;
    let mut app = test_app();
    app.display.show_all_classes = true;
    let saved = UiState::capture(&app);

    let mut restored = test_app();
    assert!(!restored.display.show_all_classes);
    saved.restore(&mut restored);
    assert!(
        restored.display.show_all_classes,
        "show_all_classes should persist across capture/restore"
    );
}

// #295 CI-events-only filter — end-to-end tests for the
// sidebar-section / toggle / persistence / count shape. The filter
// predicate itself is unit-tested in terminal::app::filter::tests.

fn ci_activity_msg(prefix: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{prefix} CI failed — abc123"),
        ts: Utc::now(),
        message_type: MessageType::Activity,
        ..Message::default()
    }
}

#[test]
fn count_ci_events_counts_failure_and_recovery_prefixes() {
    let mut app = test_app();
    app.msg.messages.push(ci_activity_msg("\u{1F534}"));
    app.msg.messages.push(ci_activity_msg("\u{1F7E2}"));
    app.msg.messages.push(chat_msg("alice", "not ci"));
    assert_eq!(app.count_ci_events(), 2);
}

#[test]
fn count_ci_events_excludes_non_activity_even_with_ci_prefix() {
    // A chat message whose text happens to start with 🔴 CI shouldn't
    // count — the predicate gates on MessageType::Activity first.
    // Prevents false-positives if someone quotes CI text in chat.
    let mut app = test_app();
    app.msg.messages.push(Message {
        from: "alice".to_string(),
        text: "\u{1F534} CI quote in chat".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    });
    assert_eq!(app.count_ci_events(), 0);
}

#[test]
fn count_ci_events_returns_zero_on_empty_log() {
    let app = test_app();
    assert_eq!(app.count_ci_events(), 0);
}

#[test]
fn toggle_ci_filter_flips_state() {
    let mut app = test_app();
    assert!(
        !app.display.ci_filter_active,
        "default is off — keeps status bar quiet + chat unfiltered"
    );
    app.toggle_ci_filter();
    assert!(app.display.ci_filter_active);
    app.toggle_ci_filter();
    assert!(!app.display.ci_filter_active);
}

#[test]
fn toggle_ci_filter_dirties_filter_index() {
    // The filter index is cached for render cost; toggling must
    // invalidate so the chat pane re-filters on the next tick.
    // Without this, the filter would flip but chat wouldn't
    // actually update until some other mutation dirtied the index.
    let mut app = test_app();
    app.msg.filter_dirty = false;
    app.toggle_ci_filter();
    assert!(app.msg.filter_dirty, "toggle must mark filter dirty");
}

#[test]
fn sidebar_hides_ci_section_when_no_ci_events() {
    // Empty state: section absent per Nielsen #8 minimalism. A
    // "CI events (0)" row would be noise when there's nothing to
    // filter to + no way to reach it.
    let mut app = test_app();
    app.sidebar.open = true;
    app.sidebar.project_expanded = true;
    let channels = app.sidebar_channels();
    assert!(
        !channels.iter().any(|c| c.contains("CI events")),
        "CI events section should be hidden when count is 0; got: {channels:?}"
    );
}

#[test]
fn sidebar_shows_ci_section_with_count_when_events_present() {
    let mut app = test_app();
    app.sidebar.open = true;
    app.sidebar.project_expanded = true;
    app.msg.messages.push(ci_activity_msg("\u{1F534}"));
    app.msg.messages.push(ci_activity_msg("\u{1F7E2}"));
    app.msg.messages.push(ci_activity_msg("\u{1F534}"));
    let channels = app.sidebar_channels();
    let ci_row = channels
        .iter()
        .find(|c| c.contains("CI events"))
        .expect("CI events section should be present when count > 0");
    assert!(
        ci_row.contains("(3)"),
        "CI events row should show count; got: `{ci_row}`"
    );
}

#[test]
fn ui_state_persists_ci_filter_active() {
    // Parallel to show_all_classes persistence — filter state
    // survives restart so Ryan doesn't lose a filtered view when
    // the TUI reloads or the session reconnects.
    use crate::terminal::ui_state::UiState;
    let mut app = test_app();
    app.display.ci_filter_active = true;
    let saved = UiState::capture(&app);

    let mut restored = test_app();
    assert!(!restored.display.ci_filter_active);
    saved.restore(&mut restored);
    assert!(
        restored.display.ci_filter_active,
        "ci_filter_active should persist across capture/restore"
    );
}

#[test]
fn to_command_sets_explicit_privacy_flag() {
    // Verifying that /to tags the message as explicit privacy so /share
    // later refuses to propagate it. Without this flag, channel-scoped
    // messages and /to DMs look identical on the wire.
    let mut app = test_app();
    app.editor.input = "/to pm private thing".to_string();
    let msg = app.submit_message().unwrap();
    assert!(
        msg.explicit_privacy,
        "/to must set explicit_privacy=true for privacy-intent detection"
    );
    assert_eq!(msg.to, Some(vec!["pm".to_string()]));
}

// ─────────────────────────────────────────────────────────────────────
// #164 — command UX cleanup tests
// ─────────────────────────────────────────────────────────────────────

/// `/help` returns a non-empty command list inline (acceptance criterion).
#[test]
fn help_command_lists_commands() {
    let mut app = test_app();
    app.editor.input = "/help".to_string();
    app.submit_message();
    let last = app
        .msg
        .messages
        .last()
        .expect("/help should post a message");
    assert!(
        last.text.starts_with("Commands:"),
        "/help output should start with 'Commands:' header, got: {}",
        last.text
    );
    // Every registered command should appear in the help output.
    for entry in SLASH_COMMANDS {
        assert!(
            last.text.contains(entry.name),
            "/help should include {name}, got: {}",
            last.text,
            name = entry.name,
        );
    }
}

/// Each SLASH_COMMANDS entry carries a function pointer, so the
/// compiler already enforces "every registered command has a
/// handler." This test locks in the discoverability invariant:
/// dispatch via submit_message produces the same side effect as
/// calling the handler directly, for every entry. Catches drift if
/// someone bypasses the registry and re-adds a stray match arm.
#[test]
fn every_registered_slash_command_dispatches_through_the_registry() {
    use super::input::SLASH_COMMANDS;
    for entry in SLASH_COMMANDS {
        // Use a representative invocation per command — most
        // commands are argument-tolerant in the empty case, so the
        // bare name is a safe probe. /status, /to, /join are the
        // only ones that would misbehave on empty args, and their
        // bare forms still route through the registry (showing
        // usage or a no-op), which is what we're verifying.
        let mut app = test_app();
        app.editor.input = entry.name.to_string();
        // submit_message returning without panic and clearing input
        // is the observable proof that the registry resolved the
        // command — `handle_unknown_command` would have left an
        // "Unknown command" system message instead.
        app.submit_message();
        let last = app.msg.messages.last();
        if let Some(m) = last {
            assert!(
                !m.text.contains("Unknown command"),
                "{} should be routed by the registry, not fall through to unknown-command \
                 (make sure it's in SLASH_COMMANDS)",
                entry.name
            );
        }
    }
}

/// `/help` is sourced from `SLASH_COMMANDS` so it can never drift from
/// the `?` keybind overlay (#164 single source of truth requirement).
#[test]
fn help_command_includes_help_itself() {
    let mut app = test_app();
    app.editor.input = "/help".to_string();
    app.submit_message();
    let last = app.msg.messages.last().unwrap();
    assert!(
        last.text.contains("/help"),
        "/help should list itself (it's in SLASH_COMMANDS)"
    );
}

/// Each command that previously had a `*_bare` sibling now handles the
/// empty-args case from a single dispatch arm. Verifies that the
/// dispatcher routes the bare invocation through the same handler as
/// the with-args invocation, and that the handler returns its usage
/// string when args are missing.
///
/// This is the regression guard for the #164 collapse: if anyone tries
/// to re-add a `*_bare` arm to the dispatcher, this test still passes
/// (it's a behavior test, not a structural test) — the structural
/// guarantee comes from the dispatch table itself, which now has 1 arm
/// per command.
#[test]
fn bare_commands_show_usage_from_single_handler() {
    let cases = [
        ("/to", "/to"),
        ("/join", "/join"),
        ("/accept", "/accept"),
        ("/share", "/share"),
        ("/search", "/search"),
    ];
    for (cmd, expected_prefix) in cases {
        let mut app = test_app();
        app.editor.input = cmd.to_string();
        app.submit_message();
        let last = app.msg.messages.last().unwrap();
        assert!(
            last.text.contains(expected_prefix),
            "{cmd} ({} chars) bare invocation should show usage containing '{expected_prefix}', got: {}",
            cmd.len(),
            last.text
        );
    }
}

// #294 freeze-fix: `request_board_refresh_if_stale` must be non-blocking
// on the event loop thread. Before the fix it called `fetch_board_stories`
// synchronously, which shelled gh CLI on the tokio runtime's main thread
// and froze the TUI under slow gh. Post-fix it signals via
// `board_fetch_request_tx` and returns; the event loop listener runs the
// fetch off-thread via `spawn_blocking`.
#[test]
fn request_board_refresh_if_stale_signals_fetch_when_board_empty() {
    let mut app = test_app();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    app.board_fetch_request_tx = Some(tx);
    assert!(
        app.board.stories.is_empty(),
        "test precondition: fresh app has no stories"
    );

    app.request_board_refresh_if_stale();

    // Signal fires on the channel; the caller returns without waiting
    // for gh. This is the contract: the event-loop thread is never
    // blocked on a subprocess call here. Previously this method called
    // `fetch_board_stories()` inline and spent the whole gh latency in
    // a single sync call.
    assert!(
        rx.try_recv().is_ok(),
        "empty board should trigger a fetch request signal"
    );
}

#[test]
fn request_board_refresh_if_stale_skips_fetch_when_board_populated() {
    use super::board::{BoardStory, StoryState};
    let mut app = test_app();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    app.board_fetch_request_tx = Some(tx);
    // Simulate a populated board — e.g., stories already loaded from
    // a prior fetch or from the poller snapshot.
    app.board.stories = vec![BoardStory {
        number: 1,
        title: "existing".to_string(),
        state: StoryState::Ready,
        body: String::new(),
        assignees: vec![],
        created: String::new(),
        reason: String::new(),
        labels: vec![],
    }];

    app.request_board_refresh_if_stale();

    // Already-populated board means no fetch needed. Without this
    // guard the method would over-fire on every view open; the poller
    // already owns ambient refresh cadence.
    assert!(
        rx.try_recv().is_err(),
        "populated board should not re-trigger fetch; got signal anyway"
    );
}

#[test]
fn request_board_refresh_if_stale_is_noop_when_no_fetch_sender() {
    let mut app = test_app();
    // `board_fetch_request_tx` is None by default — simulates headless
    // mode or pre-event-loop state. Calling the method must not panic
    // or block; it silently skips the fetch signal.
    assert!(app.board_fetch_request_tx.is_none());

    app.request_board_refresh_if_stale();
    // No assertion needed beyond "didn't panic." The prior code path
    // would have called `fetch_board_stories()` synchronously here,
    // potentially shelling gh even without a live event loop — noise
    // in test runs.
}
