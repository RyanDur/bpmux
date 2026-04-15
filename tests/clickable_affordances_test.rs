//! End-to-end tests for #183 clickable-affordance slice 1:
//!
//! Covers the static discoverability wins — the `≡` menu glyph at
//! the top-left of the chat area when the sidebar is closed, and
//! its narrowed click zone. Driven through the real PTY harness so
//! the render path is exercised end-to-end (icons module → ui.rs
//! overlay → captured frame → mouse dispatch).
//!
//! Named in behavior terms per the test-name discipline.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

/// Check whether the rendered TUI currently shows the sidebar's
/// " channels " block. The sidebar block title is the cleanest
/// sidebar-state signal — see `feedback_test_name_matches_content.md`
/// and the iteration history in `simulate_key_modifiers_test.rs` for
/// why this beats broader signatures like "left 24 cols."
fn sidebar_open(tui: &str) -> bool {
    tui.contains("channels")
}

/// Whether the `≡` menu glyph is visible on the current screen.
/// Codepoint U+2261. Present when the sidebar is closed; gone when
/// the sidebar is open (no need to advertise a toggle the user
/// already sees).
fn menu_glyph_visible(tui: &str) -> bool {
    tui.contains('\u{2261}')
}

/// GIVEN bpmux starts with the default (sidebar closed) state
/// THEN the `≡` menu glyph is visible at the top-left of the chat
///   area, giving the previously invisible click zone a visible
///   affordance
#[test]
fn menu_glyph_is_visible_when_sidebar_is_closed_by_default() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    let screen = bpmux.tui();
    assert!(
        !sidebar_open(&screen),
        "sidebar should start closed by default. screen:\n{screen}"
    );
    assert!(
        menu_glyph_visible(&screen),
        "≡ menu glyph should be visible when sidebar is closed. screen:\n{screen}"
    );
}

/// GIVEN the menu glyph is visible
/// WHEN the user presses Alt+1 (sidebar toggle keybind)
/// THEN the sidebar opens
/// AND the menu glyph disappears (the sidebar's own visible border
///   replaces the need for a separate affordance)
#[test]
fn menu_glyph_disappears_when_sidebar_opens() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    // Glyph visible at startup (sidebar closed).
    assert!(menu_glyph_visible(&bpmux.tui()));

    // Open sidebar via Alt+1 through the REST endpoint (tests the
    // same path #170's end-to-end tests exercise).
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1", "modifiers": ["alt"]}))
            .send()
            .await
            .expect("POST /api/simulate/key");
    });
    bpmux
        .wait_until("sidebar to open", Duration::from_secs(5), || {
            sidebar_open(&bpmux.tui())
        })
        .expect("sidebar should be open after Alt+1");
    let after = bpmux.tui();
    // When the sidebar is open, the menu glyph is not rendered —
    // the sidebar's own " channels " header IS the affordance.
    assert!(
        !after.starts_with('\u{2261}')
            && after
                .lines()
                .next()
                .is_some_and(|l| !l.contains('\u{2261}')),
        "menu glyph should NOT appear on row 0 when sidebar is open. screen:\n{after}"
    );
}

/// GIVEN bpmux is running (default sidebar-closed state)
/// THEN the chat title displays a `▸` expand glyph signalling the
///   project view is collapsed and clickable to open
#[test]
fn chat_title_shows_expand_glyph_when_project_view_is_collapsed() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    let screen = bpmux.tui();
    // `▸` (U+25B8) = EXPAND in icons.rs. Sits in the chat block's
    // title bar, visible in the top border region.
    assert!(
        screen.contains('\u{25B8}'),
        "expected ▸ expand glyph in chat title; screen:\n{screen}"
    );
}

/// GIVEN the sidebar is open (after Alt+1)
/// THEN the sidebar title includes the `▾` collapse glyph
#[test]
fn sidebar_title_shows_collapse_glyph_when_open() {
    let bpmux = TestBpmux::start();
    let port = bpmux.port();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let client = reqwest::Client::new();
        client
            .post(format!("http://127.0.0.1:{port}/api/simulate/key"))
            .json(&serde_json::json!({"key": "1", "modifiers": ["alt"]}))
            .send()
            .await
            .expect("POST /api/simulate/key");
    });
    bpmux
        .wait_until("sidebar to open", Duration::from_secs(5), || {
            sidebar_open(&bpmux.tui())
        })
        .expect("sidebar should be open");
    let screen = bpmux.tui();
    // `▾` (U+25BE) = COLLAPSE in icons.rs. Must appear in the sidebar
    // title area (first few lines of the screen).
    let first_lines: String = screen.lines().take(3).collect::<Vec<_>>().join("\n");
    assert!(
        first_lines.contains('\u{25BE}'),
        "expected ▾ collapse glyph in sidebar title; first lines:\n{first_lines}"
    );
}

/// GIVEN the help overlay is visible (from `?`)
/// THEN the overlay includes the click-or-press hint in its footer
#[test]
fn help_overlay_includes_click_or_keypress_dismiss_hint() {
    let bpmux = TestBpmux::start();
    bpmux.type_chars("?");
    assert!(
        bpmux.wait_for_tui(
            "click anywhere or press any key to close",
            Duration::from_secs(5)
        ),
        "help overlay should include the click-or-press dismiss hint. screen:\n{}",
        bpmux.tui()
    );
}
