//! End-to-end test for #191 topic channels.
//!
//! Tests the full topic lifecycle: create with participants,
//! send messages in topic, verify topic filtering, switch between
//! topics.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod harness;

use harness::pty::TestBpmux;
use std::time::Duration;

async fn post_chat(port: u16, from: &str, text: &str) {
    let client = reqwest::Client::new();
    client
        .post(format!("http://127.0.0.1:{port}/api/messages"))
        .header("Content-Type", "application/json")
        .body(format!(r#"{{"from":"{from}","text":"{text}"}}"#))
        .send()
        .await
        .expect("POST /api/messages");
}

/// GIVEN a user creates a topic
/// WHEN they type a message in the topic
/// THEN the message displays with the topic tag, not [broadcast]
#[test]
fn topic_messages_display_with_topic_tag() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_line("/topic create testing");
    std::thread::sleep(Duration::from_millis(300));
    // Verify we're in the topic
    let screen = bpmux.tui();
    assert!(
        screen.contains("(testing)"),
        "input prompt should show topic name. screen:\n{screen}"
    );
    // Send a message in the topic
    bpmux.type_line("hello from testing topic");
    std::thread::sleep(Duration::from_millis(300));
    let screen = bpmux.tui();
    assert!(
        screen.contains("[testing]"),
        "message should display with [testing] tag. screen:\n{screen}"
    );
    assert!(
        screen.contains("hello from testing topic"),
        "message content should be visible. screen:\n{screen}"
    );
}

/// GIVEN a topic exists with messages
/// WHEN the user switches to general
/// THEN topic-specific messages are not visible
/// AND broadcast messages are still visible
#[test]
fn switching_topics_filters_messages() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    // Send a broadcast message first
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        post_chat(bpmux.port(), "dev:claude-code", "broadcast-visible").await;
    });
    assert!(
        bpmux.wait_for_tui("broadcast-visible", Duration::from_secs(3)),
        "broadcast message should be visible"
    );

    // Create topic and send a topic message
    bpmux.type_line("/topic create scroll-fix");
    std::thread::sleep(Duration::from_millis(300));
    bpmux.type_line("topic-only-msg");
    std::thread::sleep(Duration::from_millis(300));
    let in_topic = bpmux.tui();
    assert!(
        in_topic.contains("topic-only-msg"),
        "should see topic message while in topic"
    );
    // Broadcast messages should ALSO be visible in topic view
    assert!(
        in_topic.contains("broadcast-visible"),
        "broadcast should be visible in topic view too"
    );

    // Switch back to general
    bpmux.type_line("/topic general");
    std::thread::sleep(Duration::from_millis(300));
    let in_general = bpmux.tui();
    // Topic-specific message should NOT be visible in general
    assert!(
        !in_general.contains("topic-only-msg"),
        "topic message should be hidden in general view. screen:\n{in_general}"
    );
    // Broadcast should still be visible
    assert!(
        in_general.contains("broadcast-visible"),
        "broadcast should still be visible in general"
    );
}

/// GIVEN a topic is created
/// WHEN the sidebar is opened
/// THEN it shows "topics" title and the topic names
/// AND the active topic has a ● marker
#[test]
fn sidebar_shows_topics_when_topics_exist() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    // Create a topic so sidebar switches to topic mode
    bpmux.type_line("/topic create scroll-fix");
    std::thread::sleep(Duration::from_millis(300));

    // Open sidebar (Alt+1 = Esc then 1 in raw terminal)
    bpmux.type_key("Alt-1");
    std::thread::sleep(Duration::from_millis(300));

    let screen = bpmux.tui();
    assert!(
        screen.contains("topics"),
        "sidebar title should say 'topics'. screen:\n{screen}"
    );
    assert!(
        screen.contains("scroll-fix"),
        "sidebar should show the created topic. screen:\n{screen}"
    );
    assert!(
        screen.contains("general"),
        "sidebar should show general topic. screen:\n{screen}"
    );
    // Active topic (scroll-fix) should have ● marker
    assert!(
        screen.contains("● scroll-fix") || screen.contains("●  scroll-fix"),
        "active topic should have ● marker. screen:\n{screen}"
    );
}

/// GIVEN a topic is created
/// WHEN /topic is run with no args
/// THEN the topic list shows the created topic
#[test]
fn topic_list_shows_created_topics() {
    let bpmux = TestBpmux::start();
    assert!(
        bpmux.wait_for_tui("solo", Duration::from_secs(5)),
        "TUI should render"
    );
    bpmux.type_line("/topic create my-topic pm,designer");
    std::thread::sleep(Duration::from_millis(300));
    bpmux.type_line("/topic");
    std::thread::sleep(Duration::from_millis(300));
    let screen = bpmux.tui();
    assert!(
        screen.contains("my-topic"),
        "topic list should include my-topic. screen:\n{screen}"
    );
    assert!(
        screen.contains("general"),
        "topic list should include general. screen:\n{screen}"
    );
}
