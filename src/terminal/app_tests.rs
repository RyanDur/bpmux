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
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    }
}

fn directed_msg(from: &str, text: &str, to: Vec<&str>) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: Some(to.into_iter().map(String::from).collect()),
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    }
}

fn system_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    }
}

fn activity_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Activity,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    }
}

#[test]
fn submits_message_and_clears_input() {
    let mut app = test_app();
    app.input = "hello world".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.from, "ryan:human");
    assert_eq!(msg.text, "hello world");
    assert!(app.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
}

#[test]
fn ignores_empty_input() {
    let mut app = test_app();
    app.input = "   ".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
}

#[test]
fn quit_command_sets_flag() {
    let mut app = test_app();
    app.input = "/quit".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.should_quit);
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
    app.input = "hello".to_string();
    app.submit_message();
    app.input = "world".to_string();
    app.submit_message();

    app.history_up();
    assert_eq!(app.input, "world");
    assert_eq!(app.history_index, Some(1));

    app.history_up();
    assert_eq!(app.input, "hello");
    assert_eq!(app.history_index, Some(0));
}

#[test]
fn down_arrow_returns_to_empty_after_recalling() {
    let mut app = test_app();
    app.input = "hello".to_string();
    app.submit_message();

    app.history_up();
    assert_eq!(app.input, "hello");

    app.history_down();
    assert!(app.input.is_empty());
    assert_eq!(app.history_index, None);
}

#[test]
fn history_caps_at_50_entries() {
    let mut app = test_app();
    for i in 0..60 {
        app.input = format!("msg {i}");
        app.submit_message();
    }
    assert_eq!(app.input_history.len(), MAX_HISTORY);
    assert_eq!(app.input_history[0], "msg 10");
    assert_eq!(app.input_history[49], "msg 59");
}

#[test]
fn who_command_lists_peer_name_and_returns_none() {
    let mut app = test_app();
    app.input = "/who".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
    assert_eq!(app.msg.messages[0].from, "bpmux");
    assert!(app.msg.messages[0].text.contains("ryan (you)"));
}

#[test]
fn who_command_includes_peer_activity() {
    let mut app = test_app();
    app.update_activity("dev".to_string(), "working".to_string());
    app.input = "/who".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("dev [working]"));
}

#[test]
fn unrecognized_command_adds_error_message_without_broadcast() {
    let mut app = test_app();
    app.input = "/foo".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.input.is_empty());
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

    app.input = "/debug".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.display.show_system_messages);
    assert!(app.input.is_empty());
    assert_eq!(app.msg.messages.len(), 1);
    assert!(app.msg.messages[0].text.contains("Debug mode on"));
    assert!(app.msg.messages[0].text.contains("visible"));

    app.input = "/debug".to_string();
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
    app.input = "/to dev hello there".to_string();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.from, "ryan:human");
    assert_eq!(msg.text, "hello there");
    assert_eq!(msg.to, Some(vec!["dev".to_string()]));
}

#[test]
fn to_csv_recipients_returns_directed_message() {
    let mut app = test_app();
    app.input = "/to dev,designer check this".to_string();
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
    app.input = "/to".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("/"));
}

#[test]
fn to_recipient_no_body_shows_usage() {
    let mut app = test_app();
    app.input = "/to dev".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.msg.messages[0].text.contains("/"));
}

#[test]
fn tab_completes_participant_name_after_to() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.update_activity("designer:claude-code".to_string(), "idle".to_string());
    app.input = "/to de".to_string();
    app.cursor_pos = app.input.len();
    assert!(app.tab_complete());
    // Should complete to first alphabetical match
    assert!(
        app.input.starts_with("/to de"),
        "should complete a 'de' prefix, got: {}",
        app.input
    );
    assert!(
        app.input.ends_with(' '),
        "should add trailing space for body"
    );
}

#[test]
fn tab_completes_csv_second_recipient() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.update_activity("pm:claude-code".to_string(), "idle".to_string());
    app.input = "/to dev,p".to_string();
    app.cursor_pos = app.input.len();
    assert!(app.tab_complete());
    assert_eq!(app.input, "/to dev,pm ");
}

#[test]
fn tab_does_nothing_without_to_prefix() {
    let mut app = test_app();
    app.input = "hello".to_string();
    app.cursor_pos = app.input.len();
    assert!(!app.tab_complete());
    assert_eq!(app.input, "hello");
}

#[test]
fn tab_does_nothing_in_body_portion() {
    let mut app = test_app();
    app.update_activity("dev:claude-code".to_string(), "working".to_string());
    app.input = "/to dev hel".to_string();
    app.cursor_pos = app.input.len();
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
    app.input = "got it".to_string();
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

#[test]
fn popup_matches_filters_by_typed_prefix() {
    let mut app = test_app();
    app.input = "/wh".to_string();
    let m = app.popup_matches();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, "/who");
}

#[test]
fn popup_matches_returns_all_on_bare_slash() {
    let mut app = test_app();
    app.input = "/".to_string();
    assert_eq!(app.popup_matches().len(), SLASH_COMMANDS.len());
}

#[test]
fn popup_matches_empty_after_space() {
    let mut app = test_app();
    app.input = "/to dev".to_string();
    assert!(app.popup_matches().is_empty());
}

#[test]
fn popup_navigation_wraps_around() {
    let mut app = test_app();
    app.input = "/".to_string();
    let count = app.popup_matches().len();

    // Down from None starts at 0
    app.popup_down();
    assert_eq!(app.popup_selected, Some(0));

    // Up from 0 wraps to last
    app.popup_up();
    assert_eq!(app.popup_selected, Some(count - 1));

    // Down from last wraps to 0
    app.popup_down();
    assert_eq!(app.popup_selected, Some(0));
}

#[test]
fn popup_accept_fills_input() {
    let mut app = test_app();
    app.input = "/t".to_string();
    app.popup_down(); // select /to
    assert!(app.popup_accept());
    assert_eq!(app.input, "/to ");
    assert_eq!(app.cursor_pos, 4);
    assert!(app.popup_selected.is_none());
}

#[test]
fn popup_selected_resets_on_input_change() {
    let mut app = test_app();
    app.input = "/".to_string();
    app.cursor_pos = 1;
    app.popup_down();
    assert!(app.popup_selected.is_some());
    app.insert_char('t');
    assert!(app.popup_selected.is_none());
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
    app.input = "hello dev".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message().unwrap();
    assert_eq!(msg.to, Some(vec!["dev".to_string()]));
}

#[test]
fn no_send_target_broadcasts() {
    let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
    app.input = "hello everyone".to_string();
    app.cursor_pos = app.input.len();
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
fn toggle_expands_long_system_message() {
    let mut app = test_app();
    // Add a long message from "bpmux" (like roadmap output)
    let long_text = "x".repeat(1000); // ~13 wrapped lines at width 78
    app.msg.messages.push(Message {
        from: "bpmux".into(),
        text: long_text,
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    });
    let key = app.msg.messages.last().unwrap().ts.to_rfc3339();
    assert!(
        !app.msg.expanded_messages.contains(&key),
        "should not be expanded initially"
    );
    app.toggle_last_truncated();
    assert!(
        app.msg.expanded_messages.contains(&key),
        "should be expanded after toggle"
    );
    app.toggle_last_truncated();
    assert!(
        !app.msg.expanded_messages.contains(&key),
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
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
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
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
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
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
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
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
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
    app.input = "/session".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message();
    assert!(
        msg.is_none(),
        "session command produces no broadcast message"
    );
    assert!(
        matches!(
            app.p2p_command,
            Some(crate::transport::peer::P2pCommand::CreateSession)
        ),
        "should set P2P command"
    );
}

#[test]
fn join_command_sets_p2p_command_with_args() {
    let mut app = test_app();
    app.input = "/join blue-tiger-42 someOfferCode123".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    match &app.p2p_command {
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
    app.input = "/join".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.p2p_command.is_none(), "no command without args");
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("/"));
}

#[test]
fn accept_command_sets_p2p_command() {
    let mut app = test_app();
    app.input = "/accept someAnswerCode456".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    match &app.p2p_command {
        Some(crate::transport::peer::P2pCommand::AcceptAnswer { answer_code }) => {
            assert_eq!(answer_code, "someAnswerCode456");
        }
        other => panic!("expected AcceptAnswer, got {other:?}"),
    }
}

#[test]
fn accept_without_args_shows_usage() {
    let mut app = test_app();
    app.input = "/accept".to_string();
    app.cursor_pos = app.input.len();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.p2p_command.is_none());
    let last = app.msg.messages.last().unwrap();
    assert!(last.text.contains("/"));
}

// ---------- #160: /share command ----------

#[test]
fn share_bare_shows_usage() {
    let mut app = test_app();
    app.input = "/share".to_string();
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
    app.input = "/share all hello team".to_string();
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
    app.input = "/share dev,designer coordinating on story X".to_string();
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
    app.input = "/share dev".to_string();
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
    app.input = "/share all".to_string();
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
    app.input = "/share all".to_string();
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
    app.input = "/share designer".to_string();
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
    app.input = "/share all".to_string();
    let msg = app.submit_message().unwrap();
    // Source scope in the marker should be the channel_name of the source
    assert!(
        msg.text.contains("from designer+dev+pm thread]"),
        "expected designer+dev+pm source scope, got: {}",
        msg.text
    );
}

// ---------- #161: classifier view filter ----------

#[test]
fn all_command_toggles_filter_state() {
    let mut app = test_app();
    assert!(
        !app.display.show_all_classes,
        "filter default is ON (show_all_classes false)"
    );
    app.input = "/all".to_string();
    app.submit_message();
    assert!(
        app.display.show_all_classes,
        "first /all toggles to showing everything"
    );
    app.input = "/all".to_string();
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
    app.input = "/all".to_string();
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

#[test]
fn to_command_sets_explicit_privacy_flag() {
    // Verifying that /to tags the message as explicit privacy so /share
    // later refuses to propagate it. Without this flag, channel-scoped
    // messages and /to DMs look identical on the wire.
    let mut app = test_app();
    app.input = "/to pm private thing".to_string();
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
    app.input = "/help".to_string();
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
    for (name, _, _) in SLASH_COMMANDS {
        assert!(
            last.text.contains(name),
            "/help should include {name}, got: {}",
            last.text
        );
    }
}

/// `/help` is sourced from `SLASH_COMMANDS` so it can never drift from
/// the `?` keybind overlay (#164 single source of truth requirement).
#[test]
fn help_command_includes_help_itself() {
    let mut app = test_app();
    app.input = "/help".to_string();
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
        app.input = cmd.to_string();
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
