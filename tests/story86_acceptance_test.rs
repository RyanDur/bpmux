//! Story 86 Acceptance Test: Wrapped message lines align with text, not left edge.
//!
//! AC1: Continuation lines indent to match sender prefix

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a message longer than the viewport width
/// WHEN the message wraps to the next line
/// THEN the continuation line starts at the same column as the message text
#[test]
fn ac1_wrapped_lines_align_with_text() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        // Send a message long enough to wrap in an 80-column terminal.
        // With sidebar closed, inner width is ~78 chars (80 - 2 borders).
        // Prefix "[broadcast] dev: " is 18 chars, so text starts at col 18.
        // A message of ~80 chars will definitely wrap.
        dev.send("this is a long message that should wrap to the next line and the continuation should be indented to align with the text start").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    assert!(
        bpmux.wait_for_tui("continuation", Duration::from_secs(3)),
        "Message should appear. Screen:\n{}",
        bpmux.tui()
    );

    let screen = bpmux.tui();
    let lines: Vec<&str> = screen.lines().collect();

    // Find the line containing the sender prefix "dev:"
    let msg_line_idx = lines
        .iter()
        .position(|l| l.contains("dev:"))
        .expect("should find dev: line");
    let msg_line = lines[msg_line_idx];

    // Find where the text starts (after "dev: ")
    let text_col = msg_line
        .find("this is")
        .expect("should find message text start");

    // The next line (continuation) should start with spaces up to text_col
    let continuation = lines[msg_line_idx + 1];
    let first_non_space = continuation.find(|c: char| !c.is_whitespace() && c != '│');

    // Strip the border character if present
    let border_offset = if continuation.starts_with('│') {
        1
    } else {
        0
    };
    let indent = first_non_space.unwrap_or(0);

    assert_eq!(
        indent - border_offset,
        text_col - border_offset,
        "Continuation should align with text start (col {text_col}).\nLine 1: {msg_line}\nLine 2: {continuation}"
    );
}
