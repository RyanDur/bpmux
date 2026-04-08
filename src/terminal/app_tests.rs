#![allow(clippy::unwrap_used, clippy::expect_used)]

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
    }
}

fn directed_msg(from: &str, text: &str, to: Vec<&str>) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: Some(to.into_iter().map(String::from).collect()),
    }
}

fn system_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
    }
}

fn activity_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Activity,
        to: None,
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
    assert_eq!(app.messages.len(), 1);
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
    app.max_scroll = 100;
    app.scroll_up(5);
    assert_eq!(app.scroll_offset, 5);
    app.scroll_up(3);
    assert_eq!(app.scroll_offset, 8);
}

#[test]
fn scroll_up_clamps_to_max_scroll() {
    let mut app = test_app();
    app.max_scroll = 10;
    app.scroll_up(15);
    assert_eq!(app.scroll_offset, 10);
}

#[test]
fn scroll_down_decreases_offset_to_zero() {
    let mut app = test_app();
    app.scroll_offset = 10;
    app.scroll_down(3);
    assert_eq!(app.scroll_offset, 7);
    app.scroll_down(10);
    assert_eq!(app.scroll_offset, 0);
}

#[test]
fn receiving_message_when_scrolled_up_increments_unread_count() {
    let mut app = test_app();
    app.scroll_offset = 5;
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.unread_count, 1);
    app.receive_message(chat_msg("bot:ws", "world"));
    assert_eq!(app.unread_count, 2);
}

#[test]
fn scrolling_to_bottom_resets_unread_count() {
    let mut app = test_app();
    app.scroll_offset = 3;
    app.unread_count = 5;
    app.scroll_down(3);
    assert_eq!(app.scroll_offset, 0);
    assert_eq!(app.unread_count, 0);
}

#[test]
fn receiving_message_at_bottom_does_not_increment_unread_count() {
    let mut app = test_app();
    assert_eq!(app.scroll_offset, 0);
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.unread_count, 0);
}

#[test]
fn incoming_message_stays_at_bottom_when_at_bottom() {
    let mut app = test_app();
    assert_eq!(app.scroll_offset, 0);
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll_offset, 0);
    assert_eq!(app.messages.len(), 1);
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
    assert_eq!(app.messages.len(), 1);
    assert_eq!(app.messages[0].from, "bpmux");
    assert!(app.messages[0].text.contains("ryan (you)"));
}

#[test]
fn who_command_includes_peer_activity() {
    let mut app = test_app();
    app.update_activity("dev".to_string(), "working".to_string());
    app.input = "/who".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.messages[0].text.contains("dev [working]"));
}

#[test]
fn unrecognized_command_adds_error_message_without_broadcast() {
    let mut app = test_app();
    app.input = "/foo".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.input.is_empty());
    assert_eq!(app.messages.len(), 1);
    assert_eq!(app.messages[0].from, "bpmux");
    assert!(app.messages[0].text.contains("Unknown command"));
    assert!(app.messages[0].text.contains("/foo"));
    assert!(app.messages[0].text.contains("?"));
}

#[test]
fn incoming_message_preserves_viewport_when_scrolled_up() {
    let mut app = test_app();
    app.scroll_offset = 5;
    app.receive_message(chat_msg("bot:ws", "hello"));
    assert_eq!(app.scroll_offset, 6);
    assert_eq!(app.messages.len(), 1);
}

#[test]
fn update_activity_adds_entry() {
    let mut app = test_app();
    app.update_activity("dev".to_string(), "working".to_string());
    assert!(app.peer_activity.contains_key("dev"));
    let (status, _) = &app.peer_activity["dev"];
    assert_eq!(status, "working");
}

#[test]
fn activity_message_does_not_appear_in_messages() {
    let mut app = test_app();
    app.receive_message(activity_msg("dev:claude-code", "working"));
    assert!(app.messages.is_empty());
    assert!(app.peer_activity.contains_key("dev"));
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
        app.peer_activity.is_empty(),
        "peer_activity should be empty on startup — only live connections populate it"
    );
}

#[test]
fn live_join_uses_short_name() {
    let mut app = test_app();
    app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
    app.receive_message(system_msg("bpmux", "dev:ws joined"));
    assert_eq!(
        app.peer_activity.len(),
        1,
        "same role via different transports should dedupe"
    );
    assert!(app.peer_activity.contains_key("dev"));
}

#[test]
fn live_leave_removes_by_short_name() {
    let mut app = test_app();
    app.receive_message(system_msg("bpmux", "dev:claude-code joined"));
    assert!(app.peer_activity.contains_key("dev"));
    app.receive_message(system_msg("bpmux", "dev:ws left"));
    assert!(
        !app.peer_activity.contains_key("dev"),
        "leave from any transport should remove"
    );
}

#[test]
fn debug_toggles_show_system_messages() {
    let mut app = test_app();
    assert!(!app.show_system_messages);

    app.input = "/debug".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.show_system_messages);
    assert!(app.input.is_empty());
    assert_eq!(app.messages.len(), 1);
    assert!(app.messages[0].text.contains("Debug mode on"));
    assert!(app.messages[0].text.contains("visible"));

    app.input = "/debug".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(!app.show_system_messages);
    assert_eq!(app.messages.len(), 2);
    assert!(app.messages[1].text.contains("Debug mode off"));
    assert!(app.messages[1].text.contains("hidden"));
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
    assert!(app.messages[0].text.contains("Usage:"));
}

#[test]
fn to_recipient_no_body_shows_usage() {
    let mut app = test_app();
    app.input = "/to dev".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(app.messages[0].text.contains("Usage:"));
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
    assert_eq!(app.dm_unread_from.len(), 1);
    assert!(app.dm_flash_until.is_some());
}

#[test]
fn broadcast_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(chat_msg("pm:claude-code", "hello all"));
    assert_eq!(app.dm_unread_from.len(), 0);
    assert!(app.dm_flash_until.is_none());
}

#[test]
fn to_all_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hello", vec!["all"]));
    assert_eq!(app.dm_unread_from.len(), 0);
}

#[test]
fn dm_to_other_does_not_increment_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hey dev", vec!["dev"]));
    assert_eq!(app.dm_unread_from.len(), 0);
}

#[test]
fn submit_clears_dm_unread() {
    let mut app = test_app();
    app.receive_message(directed_msg("pm:claude-code", "hey", vec!["ryan"]));
    assert_eq!(app.dm_unread_from.len(), 1);
    app.input = "got it".to_string();
    app.submit_message();
    assert_eq!(app.dm_unread_from.len(), 0);
    assert!(app.dm_flash_until.is_none());
}

#[test]
fn broadcast_message_adds_channel() {
    let mut app = test_app();
    assert_eq!(app.active_channels.len(), 0);
    app.receive_message(chat_msg("dev:claude-code", "hello"));
    assert_eq!(app.active_channels.len(), 1);
    assert!(app.active_channels.contains("broadcast"));
}

#[test]
fn directed_message_adds_new_channel() {
    let mut app = test_app();
    app.receive_message(chat_msg("dev:claude-code", "broadcast"));
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    assert_eq!(app.active_channels.len(), 2);
}

#[test]
fn same_channel_does_not_duplicate() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    app.receive_message(directed_msg("dev:claude-code", "also hey", vec!["ryan"]));
    assert_eq!(app.active_channels.len(), 1);
    assert!(app.active_channels.contains("dev"));
}

#[test]
fn different_senders_create_different_channels() {
    let mut app = test_app();
    app.receive_message(directed_msg("dev:claude-code", "hey", vec!["ryan"]));
    app.receive_message(directed_msg("pm:claude-code", "also hey", vec!["ryan"]));
    assert_eq!(app.active_channels.len(), 2);
    assert!(app.active_channels.contains("dev"));
    assert!(app.active_channels.contains("pm"));
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
fn view_broadcast_sets_current_view() {
    let mut app = test_app();
    app.input = "/view broadcast".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert_eq!(app.current_view, Some("broadcast".to_string()));
}

#[test]
fn view_all_clears_current_view() {
    let mut app = test_app();
    app.current_view = Some("broadcast".to_string());
    app.input = "/view all".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert_eq!(app.current_view, None);
}

#[test]
fn view_known_channel_sets_current_view() {
    let mut app = test_app();
    app.active_channels.insert("dev\u{2194}ryan".to_string());
    app.input = "/view dev\u{2194}ryan".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert_eq!(app.current_view, Some("dev\u{2194}ryan".to_string()));
}

#[test]
fn view_unknown_channel_shows_error() {
    let mut app = test_app();
    let original_view = app.current_view.clone();
    app.input = "/view fake".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert_eq!(app.current_view, original_view);
    assert!(
        app.messages
            .last()
            .unwrap()
            .text
            .contains("no channel matches")
    );
}

#[test]
fn bare_view_lists_channels() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    app.active_channels.insert("dev\u{2194}ryan".to_string());
    app.input = "/view".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    assert!(
        app.messages
            .last()
            .unwrap()
            .text
            .contains("Available channels")
    );
    assert!(app.messages.last().unwrap().text.contains("broadcast"));
}

#[test]
fn activity_messages_do_not_create_channels() {
    let mut app = test_app();
    app.receive_message(activity_msg("dev:claude-code", "busy"));
    assert_eq!(app.active_channels.len(), 0);
}

#[test]
fn popup_matches_filters_by_typed_prefix() {
    let mut app = test_app();
    app.input = "/v".to_string();
    let m = app.popup_matches();
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].0, "/view");
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
fn sidebar_channels_includes_active_channels() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    app.active_channels.insert("dev\u{2194}human".to_string());
    let channels = app.sidebar_channels();
    assert!(channels.contains(&"broadcast".to_string()));
    assert!(channels.contains(&"dev\u{2194}human".to_string()));
}

#[test]
fn sidebar_toggle_three_state_cycle() {
    let mut app = test_app();
    // Closed -> open + focused
    assert!(!app.sidebar_open);
    app.toggle_sidebar();
    assert!(app.sidebar_open);
    assert!(app.sidebar_focused);
    // Open + focused -> closed
    app.toggle_sidebar();
    assert!(!app.sidebar_open);
    assert!(!app.sidebar_focused);
    // Reopen, then unfocus (simulating channel select)
    app.toggle_sidebar();
    app.sidebar_focused = false; // simulate select unfocusing
    // Open + unfocused -> refocused
    app.toggle_sidebar();
    assert!(app.sidebar_open);
    assert!(app.sidebar_focused);
}

#[test]
fn sidebar_select_all_clears_view_and_target() {
    let mut app = test_app();
    app.current_view = Some("broadcast".to_string());
    app.send_target = Some("broadcast".to_string());
    app.sidebar_selected = 0; // project node is always first
    app.sidebar_select();
    assert!(app.current_view.is_none());
    assert!(app.send_target.is_none());
}

#[test]
fn sidebar_select_channel_sets_view_and_target() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    app.sidebar_selected = 1; // "broadcast" (sorted after project node)
    app.sidebar_select();
    assert_eq!(app.current_view, Some("broadcast".to_string()));
    assert_eq!(app.send_target, Some("broadcast".to_string()));
}

#[test]
fn sidebar_select_clears_unread() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    app.channel_unread.insert("broadcast".to_string(), 5);
    app.sidebar_selected = 1;
    app.sidebar_select();
    assert_eq!(app.channel_unread.get("broadcast"), None);
}

#[test]
fn sidebar_navigation_wraps() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    // channels: ["test-session", "broadcast"]
    app.sidebar_selected = 0;
    app.sidebar_up(); // wraps to last
    assert_eq!(app.sidebar_selected, 1);
    app.sidebar_down(); // wraps to first
    assert_eq!(app.sidebar_selected, 0);
}

#[test]
fn sidebar_close_unfocuses_but_stays_open() {
    let mut app = test_app();
    app.toggle_sidebar();
    assert!(app.sidebar_open);
    assert!(app.sidebar_focused);
    app.sidebar_close(); // Esc -- unfocuses but keeps sidebar open
    assert!(app.sidebar_open);
    assert!(!app.sidebar_focused);
}

#[test]
fn sidebar_collapse_hides_channels() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    assert!(app.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 2); // project + broadcast
    app.sidebar_collapse();
    assert!(!app.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 1); // project only
    assert_eq!(app.sidebar_selected, 0); // snapped to project node
}

#[test]
fn sidebar_expand_shows_channels() {
    let mut app = test_app();
    app.active_channels.insert("broadcast".to_string());
    app.project_expanded = false;
    assert_eq!(app.sidebar_channels().len(), 1);
    app.sidebar_expand();
    assert!(app.project_expanded);
    assert_eq!(app.sidebar_channels().len(), 2);
}

#[test]
fn sidebar_expanded_by_default() {
    let app = test_app();
    assert!(app.project_expanded);
}

#[test]
fn send_target_directs_outgoing_messages() {
    let mut app = App::new("human".to_string(), "test-session".to_string(), vec![]);
    app.send_target = Some("dev\u{2194}human".to_string());
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
    app.current_view = Some("broadcast".to_string());
    app.receive_message(directed_msg("dev:claude-code", "dm message", vec!["human"]));
    assert_eq!(*app.channel_unread.get("dev").unwrap_or(&0), 1);
}

#[test]
fn roadmap_command_adds_system_message() {
    // fetch_roadmap uses git rev-parse which works when tests run from the repo
    let mut app = test_app();
    app.input = "/roadmap".to_string();
    let msg = app.submit_message();
    assert!(msg.is_none());
    let last = app.messages.last().expect("should have a message");
    assert_eq!(last.from, "bpmux");
    // Either shows roadmap content or an unavailable message
    assert!(last.text.contains("bpmux Roadmap") || last.text.contains("Roadmap unavailable"),);
}

#[test]
fn roadmap_command_clears_input() {
    let mut app = test_app();
    app.input = "/roadmap".to_string();
    app.submit_message();
    assert!(app.input.is_empty());
}

#[test]
fn toggle_expands_long_system_message() {
    let mut app = test_app();
    // Add a long message from "bpmux" (like /roadmap output)
    let long_text = "x".repeat(1000); // ~13 wrapped lines at width 78
    app.messages.push(Message {
        from: "bpmux".into(),
        text: long_text,
        ts: chrono::Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    });
    let key = app.messages.last().unwrap().ts.to_rfc3339();
    assert!(
        !app.expanded_messages.contains(&key),
        "should not be expanded initially"
    );
    app.toggle_last_truncated();
    assert!(
        app.expanded_messages.contains(&key),
        "should be expanded after toggle"
    );
    app.toggle_last_truncated();
    assert!(
        !app.expanded_messages.contains(&key),
        "should collapse after second toggle"
    );
}
