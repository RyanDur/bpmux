use super::read::{render_board, render_ready, render_story, story_line};

fn story(number: u64, title: &str, state: &str, acceptor: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        "state": state,
        "acceptor": acceptor,
    })
}

#[test]
fn render_ready_shows_count_header_and_one_line_per_ready_story() {
    let ready = vec![
        story(
            1656,
            "[Feature] Broadcast off Channels",
            "ready",
            Some("owner"),
        ),
        story(1683, "[Feature] Scoped board-read", "ready", Some("team")),
    ];
    let out = render_ready(&ready);
    assert!(out.starts_with("ready (2):\n"));
    assert!(out.contains("  1  #1656  [Feature] Broadcast off Channels  awaiting owner\n"));
    assert!(out.contains("  2  #1683  [Feature] Scoped board-read  awaiting team\n"));
}

#[test]
fn render_ready_is_one_line_per_story_not_a_body_dump() {
    let ready = vec![story(1, "title", "ready", Some("owner"))];
    let out = render_ready(&ready);
    assert_eq!(out.lines().count(), 2);
}

#[test]
fn story_line_without_acceptor_omits_the_awaiting_clause() {
    let s = story(42, "no acceptor", "ready", None);
    assert_eq!(story_line(1, &s), "  1  #42  no acceptor");
}

#[test]
fn render_story_is_a_compact_summary_with_a_full_hint_not_the_body() {
    let s = story(1518, "[Bug] Reject-modal restyle", "ready", Some("owner"));
    let out = render_story(&s);
    assert!(out.contains("#1518 [Bug] Reject-modal restyle · ready · awaiting owner"));
    assert!(out.contains("use --full for the body"));
}

#[test]
fn render_board_lists_one_line_per_story_with_state_and_acceptor() {
    let stories = vec![
        story(1, "first", "ready", Some("owner")),
        story(2, "second", "in-progress", None),
    ];
    let out = render_board(&stories);
    assert!(out.starts_with("board (2):\n"));
    assert!(
        out.contains("  1  #1  first  ready  awaiting owner\n"),
        "render_board surfaces the acceptor so `bpmux board` shows who accepts (consistent with `bpmux ready`), not None; got:\n{out}"
    );
    assert!(
        out.contains("  2  #2  second  in-progress\n"),
        "a story with no acceptor renders no awaiting clause; got:\n{out}"
    );
}
