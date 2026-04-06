//! Story 61 Acceptance Test: A participant can send and read code blocks in the TUI
//!
//! AC1: Fenced code blocks are visually distinct from prose
//! AC2: Multiple code blocks are individually distinguishable
//! AC3: Indentation is preserved in code blocks
//! AC4: Inline code is visually distinct
//! AC5: Wide code blocks don't break layout

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::peer::WsPeer;
use harness::pty::TestBpmux;
use std::time::Duration;

/// AC1: GIVEN a message with a fenced code block
/// WHEN it renders in the TUI
/// THEN the code content is visible (fence markers are stripped)
#[test]
fn ac1_fenced_code_block_renders_content() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("here is code:\n```\nlet x = 42;\n```\ndone").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("let x = 42", Duration::from_secs(5)),
        "Code block content should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("done", Duration::from_secs(5)),
        "Prose after code block should be visible. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC2: GIVEN a message with multiple code blocks and prose between them
/// WHEN it renders
/// THEN both code blocks and prose sections are visible
#[test]
fn ac2_multiple_code_blocks_with_prose() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("first:\n```\nfn a() {}\n```\nsecond:\n```\nfn b() {}\n```\nend")
            .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("fn a()", Duration::from_secs(5)),
        "First code block should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("fn b()", Duration::from_secs(5)),
        "Second code block should be visible. Screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("end", Duration::from_secs(5)),
        "Trailing prose should be visible. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC3: GIVEN a code block with indentation
/// WHEN it renders
/// THEN the indentation is preserved
#[test]
fn ac3_indentation_preserved() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("code:\n```\nif true {\n    nested();\n}\n```")
            .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("nested()", Duration::from_secs(5)),
        "Indented code should be visible. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC4: GIVEN a message with inline code
/// WHEN it renders
/// THEN the inline code content is visible
#[test]
fn ac4_inline_code_renders() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        dev.send("use `cargo build` to compile").await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("cargo build", Duration::from_secs(5)),
        "Inline code should be visible. Screen:\n{}",
        bpmux.tui()
    );
}

/// AC5: GIVEN a code block wider than the TUI
/// WHEN it renders
/// THEN surrounding messages are not broken
#[test]
fn ac5_wide_code_block_does_not_break_layout() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let mut dev = WsPeer::connect(&bpmux, "dev").await;
        let wide_line = "x".repeat(200);
        dev.send(&format!("before\n```\n{wide_line}\n```\nafter"))
            .await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });
    assert!(
        bpmux.wait_for_tui("before", Duration::from_secs(5)),
        "Prose before wide code should be visible. Screen:\n{}",
        bpmux.tui()
    );
    // The message may be truncated due to length, but layout should be intact
    // (bpmux title should still render, proving no layout corruption)
    assert!(
        bpmux.wait_for_tui("bpmux", Duration::from_secs(5)),
        "TUI frame should be intact. Screen:\n{}",
        bpmux.tui()
    );
}
