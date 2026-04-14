#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::markdown::{message_matches_channel, wrap_line_with_indent};
use super::status_bar::{STALL_THRESHOLD, parse_status_with_age};
use super::theme::{
    DRACULA_CYAN, DRACULA_GREEN, DRACULA_ORANGE, DRACULA_RED, DRACULA_YELLOW, sender_color,
};
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

    app.scroll.max = 100;
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
fn status_bar_shows_dots_when_peers_active() {
    let backend = TestBackend::new(80, 15);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![]);
    app.update_activity("dev".to_string(), "available".to_string());

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());
    assert!(
        screen.contains("dev"),
        "Should show peer dot when active. Screen:\n{screen}"
    );
    assert!(
        !screen.contains("solo"),
        "Should not show 'solo' when peers active. Screen:\n{screen}"
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
    app.scroll.unread_count = 3;

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
    app.display.show_system_messages = true;

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
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    }
}

#[test]
fn channel_view_exact_match() {
    assert!(message_matches_channel(
        &directed_msg("ryan:human", "hey", &["designer"]),
        "designer",
        "ryan"
    ));
    assert!(message_matches_channel(
        &directed_msg("designer:ws", "hi", &["ryan"]),
        "designer",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &directed_msg("designer:ws", "group", &["dev", "ryan"]),
        "designer",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &directed_msg("designer:ws", "aside", &["dev"]),
        "designer",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &directed_msg("pm:ws", "fyi", &["designer"]),
        "designer",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &chat_msg("designer:ws", "broadcast"),
        "designer",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &chat_msg("ryan:human", "broadcast"),
        "designer",
        "ryan"
    ));
}

#[test]
fn channel_view_group_exact_match() {
    assert!(message_matches_channel(
        &directed_msg("dev:ws", "hello", &["pm", "ryan"]),
        "dev+pm",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &directed_msg("dev:ws", "hello", &["ryan"]),
        "dev+pm",
        "ryan"
    ));
    assert!(!message_matches_channel(
        &directed_msg("dev:ws", "hello", &["pm", "designer", "ryan"]),
        "dev+pm",
        "ryan"
    ));
}

#[test]
fn channel_view_excludes_broadcasts() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("ryan:human", "broadcast hello everyone"),
        directed_msg("pm:claude-code", "pm to dev", &["dev"]),
        chat_msg("bpmux", "Creating P2P session"),
        chat_msg("designer:claude-code", "another broadcast"),
        directed_msg("dev:human", "dev reply to pm", &["pm"]),
    ]);
    app.current_view = Some("pm".to_string());

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    // Extract chat area content (inside the bordered box)
    let chat_lines: Vec<&str> = screen
        .lines()
        .skip_while(|l| !l.contains("bpmux"))
        .skip(1)
        .take_while(|l| !l.starts_with('└'))
        .collect();
    let chat_text = chat_lines.join("\n");

    assert!(
        chat_text.contains("pm to dev"),
        "Directed message should appear. Chat:\n{chat_text}"
    );
    assert!(
        chat_text.contains("dev reply"),
        "Dev's reply should appear. Chat:\n{chat_text}"
    );
    assert!(
        !chat_text.contains("broadcast hello"),
        "User broadcast should NOT appear in channel view. Chat:\n{chat_text}"
    );
    assert!(
        !chat_text.contains("P2P session"),
        "bpmux system message should NOT appear in channel view. Chat:\n{chat_text}"
    );
    assert!(
        !chat_text.contains("another broadcast"),
        "Designer broadcast should NOT appear in channel view. Chat:\n{chat_text}"
    );
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

#[test]
fn table_renders_within_chat_width() {
    let backend = TestBackend::new(80, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let table_text = "Here is a table:\n\n| Header A | Header B |\n|---|---|\n| Short | Longer text that should wrap within cell boundaries without breaking the grid |\n| Another | Cell content here |\n";
    let mut app = test_app_with(vec![chat_msg("bpmux", table_text)]);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    // Table borders should be visible
    assert!(
        screen.contains("┌"),
        "Should have top border. Screen:\n{screen}"
    );
    assert!(
        screen.contains("│"),
        "Should have side borders. Screen:\n{screen}"
    );
    assert!(
        screen.contains("└"),
        "Should have bottom border. Screen:\n{screen}"
    );
    // Headers should be visible
    assert!(
        screen.contains("Header A"),
        "Should show header. Screen:\n{screen}"
    );
}

#[test]
fn numbered_list_with_bold_renders_inline() {
    // Reproduce the roadmap-style numbered list that Ryan reported
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let text = "### Milestones\n\n1. **TUI reliability** -- borders, input, auto-scroll, message truncation all shipped\n   - #14-#18 all closed\n\n2. **Awareness** -- visual identity, notifications\n   - #19-#23 all closed\n\n3. **Onboarding** -- help overlay, command input";
    let mut app = test_app_with(vec![chat_msg("bpmux:ws", text)]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    // Check that "1." appears on the same line as "TUI"
    let mut found = false;
    for line in screen.lines() {
        if line.contains("1.") && line.contains("TUI") {
            found = true;
            break;
        }
    }
    assert!(found, "'1.' and 'TUI' should be on the same line");
}

#[test]
fn numbered_list_number_stays_inline() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let list_text = "1. First item with enough text to possibly wrap at narrow widths and should still keep the number inline\n2. Second item also with long text\n3. Third item";
    let mut app = test_app_with(vec![chat_msg("ryan:human", list_text)]);

    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let screen = buffer_to_string(terminal.backend().buffer());

    // The number "1." should be on the same line as "First"
    for line in screen.lines() {
        if line.contains("1.") {
            assert!(
                line.contains("First"),
                "Number '1.' should be on the same line as 'First', got: '{line}'"
            );
            break;
        }
    }

    // Also check "2." is inline with "Second"
    for line in screen.lines() {
        if line.contains("2.") {
            assert!(
                line.contains("Second"),
                "Number '2.' should be on the same line as 'Second', got: '{line}'"
            );
            break;
        }
    }
}

#[test]
fn wrap_line_keeps_number_with_content() {
    // Simulates a markdown numbered list: "  1. " prefix + long content
    let line = Line::from(vec![
        Span::raw("  1. "),
        Span::raw(
            "This is a long list item that should wrap but the number stays with the first words",
        ),
    ]);
    let indent = Span::raw("     "); // 5-char indent matching "  1. "
    let result = wrap_line_with_indent(line, &indent, 40);
    // The first line should have "  1. " + some content, not just "  1. " alone
    let first_line_text: String = result[0]
        .spans
        .iter()
        .map(|s| s.content.to_string())
        .collect();
    assert!(
        first_line_text.len() > 10,
        "Number should stay with content on first line, got: '{first_line_text}'"
    );
}

#[test]
fn fresh_busy_is_yellow_not_stalled() {
    let (color, desc) = parse_status_with_age("busy: compiling", std::time::Duration::from_secs(5));
    assert_eq!(color, DRACULA_YELLOW);
    assert_eq!(desc.as_deref(), Some("compiling"));
}

#[test]
fn busy_past_threshold_flips_to_stalled_orange() {
    let age = STALL_THRESHOLD + std::time::Duration::from_secs(1);
    let (color, desc) = parse_status_with_age("busy: compiling", age);
    assert_eq!(
        color, DRACULA_ORANGE,
        "status should flip to orange past the stall threshold"
    );
    assert!(
        desc.as_deref().unwrap_or("").starts_with("stalled"),
        "description should be 'stalled Xs', got {desc:?}"
    );
}

#[test]
fn help_past_threshold_also_stalls() {
    // A "help: permission prompt" that sits for >30s is exactly the #146
    // failure mode — the hook chain broke and presence would silently lie.
    // Belt-and-suspenders: the TUI notices the staleness and flips orange.
    let age = STALL_THRESHOLD + std::time::Duration::from_secs(10);
    let (color, _) = parse_status_with_age("help: permission prompt", age);
    assert_eq!(color, DRACULA_ORANGE);
}

#[test]
fn available_never_stalls_even_if_old() {
    // "available" = idle, which is the normal expected state for long
    // stretches. Aging an idle peer into "stalled" would break the UI for
    // every quiet moment.
    let (color, desc) = parse_status_with_age("available", std::time::Duration::from_secs(3600));
    assert_eq!(color, DRACULA_GREEN);
    assert!(desc.is_none());
}

#[test]
fn raw_busy_without_colon_flips_to_stalled_when_old() {
    let age = STALL_THRESHOLD + std::time::Duration::from_secs(1);
    let (color, _) = parse_status_with_age("busy", age);
    assert_eq!(color, DRACULA_ORANGE);
}

#[test]
fn raw_help_fresh_stays_red() {
    let (color, _) = parse_status_with_age("help", std::time::Duration::from_secs(2));
    assert_eq!(color, DRACULA_RED);
}

// ---------- #151: CI status indicator ----------

#[test]
fn ci_unknown_hides_indicator() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    // ci_state defaults to Unknown at App::new.
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(!content.contains("CI \u{2713}"));
    assert!(!content.contains("CI \u{2717}"));
    assert!(!content.contains("CI \u{25CB}"));
}

#[test]
fn ci_green_renders_check_glyph() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    app.ci_state = super::ci::CiState::Green;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("CI \u{2713}"),
        "expected CI ✓ in frame, got:\n{content}"
    );
}

#[test]
fn ci_red_renders_cross_glyph() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    app.ci_state = super::ci::CiState::Red;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("CI \u{2717}"),
        "expected CI ✗ in frame, got:\n{content}"
    );
}

#[test]
fn ci_running_renders_circle_glyph() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    app.ci_state = super::ci::CiState::Running;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("CI \u{25CB}"),
        "expected CI ◌ in frame, got:\n{content}"
    );
}

#[test]
fn ci_indicator_does_not_evict_help_hint_on_wide_terminal() {
    // On a wide-enough terminal, both the CI indicator and the help hint
    // fit alongside each other. Validates that the layout arithmetic
    // doesn't drop the hint when CI is present.
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    app.ci_state = super::ci::CiState::Green;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(content.contains("CI \u{2713}"));
    assert!(content.contains("?=help"));
}

// ---------- #161: classifier view filter ----------

#[test]
fn class_filter_hides_bpmux_auto_nudges_by_default() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    // Auto-nudges are identified by the 📝 marker prefix added in #163's
    // build_nudge_text. Non-nudge bpmux messages (tool output, system
    // notices) do NOT carry the marker and continue rendering normally.
    let mut app = test_app_with(vec![
        chat_msg("ryan:human", "hello"),
        chat_msg(
            "bpmux",
            "\u{1F4DD} 3 candidate moments from recent chat. Pointers...",
        ),
    ]);
    assert!(!app.display.show_all_classes);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(content.contains("hello"), "non-bpmux message should render");
    assert!(
        !content.contains("3 candidate moments"),
        "bpmux auto-nudge (📝 prefix) should be hidden by default"
    );
    assert!(
        app.display.hidden_class_count >= 1,
        "hidden count should reflect suppressed auto-nudge, got {}",
        app.display.hidden_class_count
    );
}

#[test]
fn class_filter_does_not_hide_non_nudge_bpmux_output() {
    // Regression guard: bpmux sends legitimate output (/board, /roadmap,
    // join/leave system notices, tool results) that users want to see.
    // The filter must only catch the 📝-marked auto-nudges, not all
    // bpmux-sourced content.
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("ryan:human", "hello"),
        chat_msg("bpmux", "## Board\n- #142 delivered\n- #159 ready"),
    ]);
    assert!(!app.display.show_all_classes);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("Board") || content.contains("#142"),
        "non-nudge bpmux output must render:\n{content}"
    );
}

#[test]
fn class_filter_show_all_reveals_auto_nudges() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("ryan:human", "hello"),
        chat_msg("bpmux", "\u{1F4DD} 3 candidate moments from recent chat"),
    ]);
    app.display.show_all_classes = true;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(content.contains("hello"));
    assert!(
        content.contains("3 candidate moments"),
        "auto-nudge should render when show_all_classes is on"
    );
}

#[test]
fn class_filter_status_bar_shows_hidden_count() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![
        chat_msg("ryan:human", "substantive message"),
        chat_msg("bpmux", "\u{1F4DD} auto-nudge one"),
        chat_msg("bpmux", "\u{1F4DD} auto-nudge two"),
    ]);
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("hidden"),
        "status bar should contain 'hidden' count when filter suppresses messages:\n{content}"
    );
}

#[test]
fn class_filter_status_bar_shows_showing_all() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hello")]);
    app.display.show_all_classes = true;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    assert!(
        content.contains("showing all"),
        "status bar should indicate 'showing all' when filter is off:\n{content}"
    );
}

#[test]
fn ci_indicator_fits_80x24_with_full_peer_presence() {
    // Production layout is 80x24. Previous CI tests used 120 cols which
    // gave the layout arithmetic a lot of slack. This test asserts the
    // tightest realistic production case: 80 cols, three peers visible,
    // CI state Red (longest non-whitespace form is same width as Green/
    // Running). Designer flagged this gap during #151 desk-check.
    use std::time::Instant;
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app_with(vec![chat_msg("ryan:human", "hi")]);
    // Populate three peer activity entries so the status bar renders
    // the dot cluster at its realistic width.
    app.peer_activity.insert(
        "designer".to_string(),
        ("available".to_string(), Instant::now()),
    );
    app.peer_activity
        .insert("dev".to_string(), ("available".to_string(), Instant::now()));
    app.peer_activity
        .insert("pm".to_string(), ("available".to_string(), Instant::now()));
    app.ci_state = super::ci::CiState::Red;
    terminal.draw(|frame| render(frame, &mut app)).unwrap();
    let content = buffer_to_string(terminal.backend().buffer());
    // Both the CI glyph AND the help hint must survive at 80 cols with
    // peers present — that's the real-world layout we ship.
    assert!(
        content.contains("CI \u{2717}"),
        "CI indicator missing from 80x24 frame:\n{content}"
    );
    assert!(
        content.contains("?=help"),
        "help hint evicted by CI indicator at 80x24:\n{content}"
    );
}
