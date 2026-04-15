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
    assert!(
        bpmux.wait_for_tui("(testing)", Duration::from_secs(5)),
        "input prompt should show topic name. screen:\n{}",
        bpmux.tui()
    );
    // Send a message in the topic
    bpmux.type_line("hello from testing topic");
    assert!(
        bpmux.wait_for_tui("[testing]", Duration::from_secs(5)),
        "message should display with [testing] tag. screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
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
    assert!(
        bpmux.wait_for_tui("(scroll-fix)", Duration::from_secs(5)),
        "topic prompt should show scroll-fix. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_line("topic-only-msg");
    assert!(
        bpmux.wait_for_tui("topic-only-msg", Duration::from_secs(5)),
        "topic message should render. screen:\n{}",
        bpmux.tui()
    );
    let in_topic = bpmux.tui();
    // Broadcast messages should ALSO be visible in topic view
    assert!(
        in_topic.contains("broadcast-visible"),
        "broadcast should be visible in topic view too"
    );

    // Switch back to general
    bpmux.type_line("/topic general");
    bpmux
        .wait_until(
            "topic-only-msg to be filtered out in general view",
            Duration::from_secs(5),
            || !bpmux.tui().contains("topic-only-msg"),
        )
        .expect("topic message should be hidden in general view");
    let in_general = bpmux.tui();
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
    // Create a topic so sidebar switches to topic mode. Wait for the
    // topic-created confirmation system message to render, then for
    // the prompt to show the active topic — both signals must land
    // before Alt+1 is pressed, or the keystroke races the creation.
    bpmux.type_line("/topic create scroll-fix");
    assert!(
        bpmux.wait_for_tui("Created topic 'scroll-fix'", Duration::from_secs(5)),
        "topic-created system message should render. screen:\n{}",
        bpmux.tui()
    );
    assert!(
        bpmux.wait_for_tui("(scroll-fix)", Duration::from_secs(5)),
        "topic should be active before opening sidebar. screen:\n{}",
        bpmux.tui()
    );

    // Open sidebar (Alt+1 = Esc then 1 in raw terminal). Wait for the
    // sidebar title specifically (with its box-drawing prefix) — the
    // status-bar hint "⌥1=topics" also contains "topics" and would
    // otherwise false-match before the sidebar is actually open.
    bpmux.type_key("Alt-1");
    bpmux
        .wait_until(
            "sidebar to open with topic entries",
            Duration::from_secs(5),
            || {
                let s = bpmux.tui();
                s.contains("scroll-fix") && s.contains("general") && s.contains("●")
            },
        )
        .expect("sidebar should show topic list with active marker");

    let screen = bpmux.tui();
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
    assert!(
        bpmux.wait_for_tui("(my-topic)", Duration::from_secs(5)),
        "topic should be created and active. screen:\n{}",
        bpmux.tui()
    );
    bpmux.type_line("/topic");
    // /topic with no args prints a "Topics:" header listing topics.
    // Wait for that header — the `my-topic` string alone would
    // false-match the active-topic prompt from the creation step.
    assert!(
        bpmux.wait_for_tui("Topics:", Duration::from_secs(5)),
        "/topic should print the topic list header. screen:\n{}",
        bpmux.tui()
    );
    let screen = bpmux.tui();
    assert!(
        screen.contains("general"),
        "topic list should include general. screen:\n{screen}"
    );
}

/// GIVEN ryan creates a topic "design" with pm as participant
/// WHEN ryan sends a message in "design"
/// THEN pm receives it (auto-subscribed as named participant)
/// AND dev does NOT receive it (not named as participant)
#[test]
fn auto_subscribed_participant_receives_topic_messages() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let bpmux = TestBpmux::start();
        assert!(
            bpmux.wait_for_tui("solo", Duration::from_secs(5)),
            "TUI should render"
        );

        // Connect two peers
        let mut pm = harness::peer::WsPeer::connect(&bpmux, "pm").await;
        let mut dev = harness::peer::WsPeer::connect(&bpmux, "dev").await;

        // Drain join system messages
        tokio::time::sleep(Duration::from_millis(500)).await;
        let _ = pm.recv_message_or_timeout(Duration::from_millis(500)).await;
        let _ = pm.recv_message_or_timeout(Duration::from_millis(500)).await;
        let _ = dev
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;
        let _ = dev
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;

        // Ryan creates a topic with pm (not dev) via TUI
        bpmux.type_line("/topic create design pm");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Drain the creation system messages
        let _ = pm.recv_message_or_timeout(Duration::from_millis(500)).await;
        let _ = dev
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;

        // Ryan sends a message in the topic via TUI
        bpmux.type_line("testing auto-subscribe");
        tokio::time::sleep(Duration::from_millis(500)).await;

        // pm should receive it (auto-subscribed as named participant)
        let pm_msg = pm.recv_message_or_timeout(Duration::from_secs(2)).await;
        assert!(
            pm_msg.is_some(),
            "pm should receive topic message as auto-subscribed participant"
        );

        // dev should NOT receive it
        let dev_msg = dev.recv_message_or_timeout(Duration::from_secs(1)).await;
        assert!(
            dev_msg.is_none(),
            "dev should NOT receive topic message (not a participant), got: {dev_msg:?}"
        );
    });
}

/// GIVEN pm is subscribed to topic "teaching" and dev is NOT
/// WHEN pm sends a message in "teaching"
/// THEN dev does not receive that message
/// AND pm does receive it
#[test]
fn non_participant_does_not_receive_topic_messages() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let bpmux = TestBpmux::start();
        assert!(
            bpmux.wait_for_tui("solo", Duration::from_secs(5)),
            "TUI should render"
        );

        // Connect two peers: pm (participant) and dev (non-participant)
        let mut pm = harness::peer::WsPeer::connect(&bpmux, "pm").await;
        let mut dev = harness::peer::WsPeer::connect(&bpmux, "dev").await;

        // Drain join system messages
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = pm.recv_message_or_timeout(Duration::from_millis(500)).await;
        let _ = pm.recv_message_or_timeout(Duration::from_millis(500)).await;
        let _ = dev
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;
        let _ = dev
            .recv_message_or_timeout(Duration::from_millis(500))
            .await;

        // pm subscribes to "teaching" topic
        pm.subscribe(&["teaching"]).await;
        // dev does NOT subscribe — stays on broadcast only

        // pm sends a message in the "teaching" topic
        pm.send_topic("secret teaching content", "teaching").await;

        // pm should receive it (as the sender, echoed back)
        let pm_msg = pm.recv_message_or_timeout(Duration::from_secs(2)).await;
        assert!(
            pm_msg.is_some(),
            "pm should receive their own topic message"
        );

        // dev should NOT receive it — negative test
        let dev_msg = dev.recv_message_or_timeout(Duration::from_secs(1)).await;
        assert!(
            dev_msg.is_none(),
            "dev should NOT receive topic message they're not subscribed to, but got: {dev_msg:?}"
        );

        // Verify broadcast still works for both
        pm.send("broadcast hello").await;
        let dev_broadcast = dev.recv_message_or_timeout(Duration::from_secs(2)).await;
        assert!(
            dev_broadcast.is_some(),
            "dev should still receive broadcast messages"
        );
    });
}
