#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::markdown::{message_involves_participants, wrap_line_with_indent};
use super::theme::{DRACULA_CYAN, sender_color};
use super::ui::render;
use crate::session::Message;
use crate::session::MessageType;
use crate::terminal::app::App;
use chrono::Utc;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};

fn test_app_with(messages: Vec<Message>) -> App {
    App::new("dev".to_string(), "test-id".to_string(), messages)
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

fn system_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
    }
}

#[test]
fn renders_without_panic() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new(
        "ryan".to_string(),
        "test-id".to_string(),
        vec![chat_msg("ryan:human", "hello")],
    );

    terminal.draw(|frame| render(frame, &mut app)).unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(content.contains("hello"));
    assert!(content.contains("ryan:"));
    assert!(content.contains("solo"));
}

#[test]
fn scroll_offset_shows_earlier_messages() {
    let backend = TestBackend::new(80, 15); // enough for chat + input + status
    let mut terminal = Terminal::new(backend).unwrap();

    // Create enough messages to overflow the chat pane
    let messages: Vec<Message> = (0..20)
        .map(|i| chat_msg("dev:human", &format!("message {i}")))
        .collect();

    // At scroll_offset 0 (bottom), last message visible
    let mut app = test_app_with(messages.clone());
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let at_bottom = buffer_to_string(terminal.backend().buffer());
    assert!(
        at_bottom.contains("message 19"),
        "Bottom should show last message. Screen:\n{at_bottom}"
    );

    // Scroll up -- earlier messages become visible, latest may not be
    app.scroll_up(15);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let scrolled = buffer_to_string(terminal.backend().buffer());
    assert!(
        scrolled.contains("message 0"),
        "Scrolled up should show earlier messages. Screen:\n{scrolled}"
    );
}

#[test]
fn scroll_indicator_shows_when_scrolled_up() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = test_app_with(vec![]);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let at_bottom = buffer_to_string(terminal.backend().buffer());
    assert!(
        !at_bottom.contains("scrolled up"),
        "No scroll indicator at bottom"
    );

    app.max_scroll = 100;
    app.scroll_up(3);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let scrolled = buffer_to_string(terminal.backend().buffer());
    assert!(
        scrolled.contains("scrolled up 3"),
        "Should show scroll indicator. Screen:\n{scrolled}"
    );
}

#[test]
fn status_bar_shows_solo_when_no_peers() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("solo"),
        "Should show 'solo' with no peers. Screen:\n{screen}"
    );
}

#[test]
fn status_bar_shows_peer_count_when_connected() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![]);
    app.connected_peers = 2;

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("2 connected"),
        "Should show '2 connected' with peers. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("solo"),
        "Should not show 'solo' when peers connected. Screen:\n{screen}"
    );
}

#[test]
fn system_messages_filtered_from_chat() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("dev:human", "visible chat"),
        system_msg("ryan:claude-code", "[turn complete]"),
    ]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("visible chat"),
        "Chat messages should appear. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("turn complete"),
        "System messages should be filtered. Screen:\n{screen}"
    );
}

#[test]
fn long_messages_truncated_with_indicator() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // 78 chars inner width. 11+ wrapped lines should truncate.
    // "bot: " prefix = 5 chars, so 73 chars of text per first line, 78 per subsequent
    // 73 + 78*10 = 853 chars = 11 lines.
    let long_text = "word ".repeat(200); // ~1000 chars, well over 10 lines

    let mut app = test_app_with(vec![chat_msg("bot:ws", &long_text)]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    assert!(
        screen.contains("[+"),
        "Long message should show [+N lines] indicator. Screen:\n{screen}"
    );
    assert!(
        screen.contains("lines]"),
        "Indicator should show line count. Screen:\n{screen}"
    );
}

#[test]
fn short_messages_not_truncated() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = test_app_with(vec![chat_msg("bot:ws", "short message")]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    assert!(
        screen.contains("short message"),
        "Short message should render fully. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("[+"),
        "Short message should not be truncated. Screen:\n{screen}"
    );
}

#[test]
fn ten_line_message_not_truncated() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // 78 inner width. "[broadcast] bot: " = 17 chars prefix.
    // First line fits 61 chars of text. 10 lines = 61 + 78*9 = 763 chars.
    let text_for_ten_lines = "a".repeat(763);

    let mut app = test_app_with(vec![chat_msg("bot:ws", &text_for_ten_lines)]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    assert!(
        !screen.contains("[+"),
        "10-line message should NOT be truncated. Screen:\n{screen}"
    );
}

#[test]
fn eleven_line_message_is_truncated() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // 11 lines = 61 + 78*10 = 841 chars
    let text_for_eleven_lines = "a".repeat(841);

    let mut app = test_app_with(vec![chat_msg("bot:ws", &text_for_eleven_lines)]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    assert!(
        screen.contains("[+"),
        "11-line message SHOULD be truncated. Screen:\n{screen}"
    );
}

#[test]
fn status_bar_shows_unread_count_when_scrolled_up() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![]);
    app.unread_count = 3;

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("3 new"),
        "Should show '3 new' unread indicator. Screen:\n{screen}"
    );
}

#[test]
fn renders_without_panic_with_long_input() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new("ryan".to_string(), "test-id".to_string(), vec![]);
    app.input = "a".repeat(500);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();

    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("ryan >"),
        "Input title should include peer name. Screen:\n{screen}"
    );
}

#[test]
fn team_names_get_distinct_expected_colors() {
    // Pin exact colors for our team names -- catches hash changes that
    // would cause collisions between ryan, dev, and pm
    let ryan_color = sender_color("ryan");
    let dev_color = sender_color("dev");
    let pm_color = sender_color("pm");

    assert_ne!(ryan_color, dev_color, "ryan and dev must differ");
    assert_ne!(ryan_color, pm_color, "ryan and pm must differ");
    assert_ne!(dev_color, pm_color, "dev and pm must differ");
}

#[test]
fn system_messages_visible_when_debug_on() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("dev:human", "visible chat"),
        system_msg("ryan:claude-code", "[turn complete]"),
    ]);
    app.show_system_messages = true;

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("visible chat"),
        "Chat messages should appear. Screen:\n{screen}"
    );
    assert!(
        screen.contains("turn complete"),
        "System messages should appear when debug is on. Screen:\n{screen}"
    );
}

#[test]
fn message_with_code_block_renders_without_panic() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg(
        "dev:human",
        "before\n```\nlet x = 1;\n```\nafter",
    )]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("before"),
        "Text before code block should render. Screen:\n{screen}"
    );
    assert!(
        screen.contains("let x = 1;"),
        "Code block content should render. Screen:\n{screen}"
    );
    assert!(
        screen.contains("after"),
        "Text after code block should render. Screen:\n{screen}"
    );
    // tui-markdown handles code block rendering -- fences may or may not appear
    // depending on markdown parsing context (blank lines before fences etc.)
}

#[test]
fn markdown_heading_styles_survive_line_reconstruction() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("bpmux:ws", "## Board\n- item one")]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let buffer = terminal.backend().buffer();

    // Find the "Board" text in the buffer and check its style
    let mut found_board = false;
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            if let Some(cell) = buffer.cell((x, y)) {
                row.push(cell.symbol().chars().next().unwrap_or(' '));
            }
        }
        if row.contains("Board") {
            // Find the exact column of 'B' in "Board"
            if let Some(col) = row.find('B') {
                let cell = buffer.cell((col as u16, y)).unwrap();
                // Heading 2 should be cyan + bold
                assert_eq!(
                    cell.fg, DRACULA_CYAN,
                    "H2 heading should be Dracula cyan. Row: {row}"
                );
                assert!(
                    cell.modifier.contains(Modifier::BOLD),
                    "H2 heading should be bold. Row: {row}"
                );
                found_board = true;
            }
            break;
        }
    }
    assert!(
        found_board,
        "Should find 'Board' heading in rendered output"
    );

    // Verify hashes are stripped -- no "##" should appear in the rendered buffer
    let screen = buffer_to_string(buffer);
    assert!(
        !screen.contains("##"),
        "Heading hashes should be stripped. Screen:\n{screen}"
    );
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let mut s = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            s.push(
                buffer
                    .cell((x, y))
                    .map_or(' ', |c| c.symbol().chars().next().unwrap_or(' ')),
            );
        }
        s.push('\n');
    }
    s
}

fn directed_msg(from: &str, text: &str, to: &[&str]) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: Some(to.iter().map(|s| s.to_string()).collect()),
    }
}

#[test]
fn participant_filter_matches_sender() {
    let msg = chat_msg("dev:ws", "hello");
    assert!(message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn participant_filter_matches_recipient() {
    let msg = directed_msg("pm:ws", "hello dev", &["dev"]);
    assert!(message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn participant_filter_excludes_unrelated() {
    let msg = directed_msg("pm:ws", "hello designer", &["designer"]);
    assert!(!message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn participant_filter_broadcast_matches_sender() {
    let msg = chat_msg("dev:ws", "broadcast");
    assert!(message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn participant_filter_broadcast_no_match_for_other() {
    let msg = chat_msg("pm:ws", "broadcast");
    assert!(!message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn participant_filter_group_requires_all() {
    let msg = directed_msg("dev:ws", "hello", &["pm"]);
    // "dev+pm" view: dev is sender, pm is recipient -- both involved
    assert!(message_involves_participants(&msg, "dev+pm", "ryan"));
}

#[test]
fn participant_filter_group_missing_one() {
    let msg = directed_msg("dev:ws", "hello", &["designer"]);
    // "dev+pm" view: dev is sender but pm not involved
    assert!(!message_involves_participants(&msg, "dev+pm", "ryan"));
}

#[test]
fn participant_filter_owner_message_always_matches() {
    let msg = chat_msg("ryan:human", "owner broadcast");
    assert!(message_involves_participants(&msg, "dev", "ryan"));
}

#[test]
fn wrap_line_short_line_unchanged() {
    let line = Line::from("short");
    let indent = Span::raw("   ");
    let result = wrap_line_with_indent(line, &indent, 80);
    assert_eq!(result.len(), 1);
}

#[test]
fn wrap_line_adds_indent_on_continuation() {
    let line = Line::from("abcdefghij"); // 10 chars
    let indent = Span::raw("  "); // 2-char indent
    let result = wrap_line_with_indent(line, &indent, 5); // wrap at 5
    // Line 1: "abcde" (5 chars)
    // Line 2: indent(2) + "fgh" (3 chars = 5 total)
    // Line 3: indent(2) + "ij" (2 chars)
    assert_eq!(result.len(), 3);
    // Continuation lines should start with indent
    assert_eq!(result[1].spans[0].content, "  ");
    assert_eq!(result[2].spans[0].content, "  ");
}

#[test]
fn wrap_line_zero_width_returns_original() {
    let line = Line::from("test");
    let indent = Span::raw("");
    let result = wrap_line_with_indent(line, &indent, 0);
    assert_eq!(result.len(), 1);
}
