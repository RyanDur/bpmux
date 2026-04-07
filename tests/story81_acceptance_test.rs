//! Story 81 Acceptance Test: Pasting multiline text sends as one message.
//!
//! AC1: Pasting multiline text inserts all lines into the buffer
//! AC2: Pressing Enter submits the full buffer as a single message
//! AC3: The sent message preserves the original line breaks

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1+AC2: GIVEN multiline text is pasted
/// WHEN the user presses Enter
/// THEN it sends as a single message (not multiple)
#[test]
fn ac1_ac2_paste_sends_single_message() {
    let bpmux = TestBpmux::start();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut peer = WsPeer::connect(&bpmux, "bot").await;

        // Simulate bracketed paste: \x1b[200~ ... \x1b[201~
        bpmux.paste("line one\nline two\nline three");
        std::thread::sleep(Duration::from_millis(300));

        // Press Enter to submit
        bpmux.type_line("");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // The peer should receive exactly one message containing all lines
        let (_, text) = peer.recv_message(Duration::from_secs(3)).await;
        assert!(
            text.contains("line one") && text.contains("line two") && text.contains("line three"),
            "Should receive all three lines in one message, got: {text}"
        );

        // Should NOT receive a second message (the paste wasn't split)
        let extra = peer
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;
        assert!(
            extra.is_none(),
            "Should not receive additional messages from the paste, got: {extra:?}"
        );
    });
}
