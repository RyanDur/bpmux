//! Tests for the transport-neutral `TerminalInjectEvent` type and
//! its conversion to crossterm `Event`. The REST simulate handlers
//! enqueue these; the event loop converts at the boundary so
//! transport never touches crossterm types.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::event::{InjectEventError, TerminalInjectEvent};
use crossterm::event::{Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind};

#[test]
fn click_converts_to_left_button_down_with_no_modifiers() {
    let event = TerminalInjectEvent::Click { x: 4, y: 2 }
        .into_crossterm()
        .expect("click should convert");
    match event {
        Event::Mouse(m) => {
            assert_eq!(m.kind, MouseEventKind::Down(MouseButton::Left));
            assert_eq!(m.column, 4);
            assert_eq!(m.row, 2);
            assert_eq!(m.modifiers, KeyModifiers::empty());
        }
        other => panic!("expected Mouse event, got {other:?}"),
    }
}

#[test]
fn plain_key_converts_to_keyevent_with_empty_modifiers() {
    let event = TerminalInjectEvent::Key {
        key: "1".to_string(),
        modifiers: vec![],
    }
    .into_crossterm()
    .expect("plain key should convert");
    match event {
        Event::Key(k) => {
            assert_eq!(k.code, KeyCode::Char('1'));
            assert_eq!(k.modifiers, KeyModifiers::empty());
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[test]
fn named_keys_resolve_to_their_keycodes() {
    for (name, expected) in [
        ("enter", KeyCode::Enter),
        ("esc", KeyCode::Esc),
        ("escape", KeyCode::Esc),
        ("tab", KeyCode::Tab),
        ("backspace", KeyCode::Backspace),
        ("up", KeyCode::Up),
        ("down", KeyCode::Down),
        ("left", KeyCode::Left),
        ("right", KeyCode::Right),
    ] {
        let event = TerminalInjectEvent::Key {
            key: name.to_string(),
            modifiers: vec![],
        }
        .into_crossterm()
        .unwrap_or_else(|_| panic!("`{name}` should convert"));
        match event {
            Event::Key(k) => assert_eq!(k.code, expected, "wrong keycode for `{name}`"),
            other => panic!("expected Key for `{name}`, got {other:?}"),
        }
    }
}

#[test]
fn multi_modifier_combines_bits() {
    let event = TerminalInjectEvent::Key {
        key: "x".to_string(),
        modifiers: vec!["ctrl".to_string(), "shift".to_string()],
    }
    .into_crossterm()
    .expect("ctrl+shift should convert");
    match event {
        Event::Key(k) => {
            assert_eq!(k.code, KeyCode::Char('x'));
            assert!(k.modifiers.contains(KeyModifiers::CONTROL));
            assert!(k.modifiers.contains(KeyModifiers::SHIFT));
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[test]
fn modifier_names_are_case_insensitive() {
    let event = TerminalInjectEvent::Key {
        key: "a".to_string(),
        modifiers: vec!["ALT".to_string(), "Shift".to_string()],
    }
    .into_crossterm()
    .expect("uppercase modifiers should convert");
    match event {
        Event::Key(k) => {
            assert!(k.modifiers.contains(KeyModifiers::ALT));
            assert!(k.modifiers.contains(KeyModifiers::SHIFT));
        }
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[test]
fn control_alias_resolves_to_ctrl() {
    let event = TerminalInjectEvent::Key {
        key: "c".to_string(),
        modifiers: vec!["control".to_string()],
    }
    .into_crossterm()
    .expect("control alias should convert");
    match event {
        Event::Key(k) => assert!(k.modifiers.contains(KeyModifiers::CONTROL)),
        other => panic!("expected Key event, got {other:?}"),
    }
}

#[test]
fn unknown_key_returns_error() {
    let err = TerminalInjectEvent::Key {
        key: "unknown-key".to_string(),
        modifiers: vec![],
    }
    .into_crossterm()
    .expect_err("unknown key should error");
    assert!(matches!(err, InjectEventError::UnknownKey(s) if s == "unknown-key"));
}

#[test]
fn unknown_modifier_returns_error() {
    let err = TerminalInjectEvent::Key {
        key: "1".to_string(),
        modifiers: vec!["meta".to_string()],
    }
    .into_crossterm()
    .expect_err("unknown modifier should error");
    assert!(matches!(err, InjectEventError::UnknownModifier(s) if s == "meta"));
}

#[test]
fn validate_mirrors_into_crossterm_success_and_failure() {
    let ok = TerminalInjectEvent::Key {
        key: "enter".to_string(),
        modifiers: vec!["alt".to_string()],
    };
    assert!(ok.validate().is_ok());
    assert!(ok.into_crossterm().is_ok());

    let bad_key = TerminalInjectEvent::Key {
        key: "nope".to_string(),
        modifiers: vec![],
    };
    assert!(bad_key.validate().is_err());
    assert!(bad_key.into_crossterm().is_err());

    let bad_mod = TerminalInjectEvent::Key {
        key: "a".to_string(),
        modifiers: vec!["hyper".to_string()],
    };
    assert!(bad_mod.validate().is_err());
    assert!(bad_mod.into_crossterm().is_err());

    // Click always validates — no vocabulary involved.
    let click = TerminalInjectEvent::Click { x: 0, y: 0 };
    assert!(click.validate().is_ok());
}

#[test]
fn error_messages_reference_the_offending_name() {
    let bad_key_msg = InjectEventError::UnknownKey("zoom".to_string()).message();
    assert!(bad_key_msg.contains("zoom"));
    let bad_mod_msg = InjectEventError::UnknownModifier("hyper".to_string()).message();
    assert!(bad_mod_msg.contains("hyper"));
}

// ── #268 thread-create marker parsing ──

use super::event::parse_thread_create;

#[test]
fn thread_create_marker_parses_id_name_participants() {
    let text = "__BPMUX_THREAD_CREATE__|cheese-123|cheese|ryan,designer";
    let (id, name, participants) = parse_thread_create(text).unwrap();
    assert_eq!(id, "cheese-123");
    assert_eq!(name, "cheese");
    assert_eq!(participants, vec!["ryan", "designer"]);
}

#[test]
fn thread_create_marker_with_single_participant() {
    let text = "__BPMUX_THREAD_CREATE__|solo-1|solo-review|ryan";
    let (_id, name, participants) = parse_thread_create(text).unwrap();
    assert_eq!(name, "solo-review");
    assert_eq!(participants, vec!["ryan"]);
}

#[test]
fn thread_create_marker_handles_name_with_pipes_reserved() {
    // Name "shouldn't" contain pipes in practice, but splitn(3)
    // means participants eats any remaining pipes — splits on first
    // three, so name stays clean if it was slugified.
    let text = "__BPMUX_THREAD_CREATE__|id-1|some-name|ryan,designer";
    let (_id, name, _p) = parse_thread_create(text).unwrap();
    assert_eq!(name, "some-name");
}

#[test]
fn thread_create_marker_rejects_non_marker_text() {
    assert!(parse_thread_create("hello world").is_none());
    assert!(parse_thread_create("__BPMUX_VIEW_TOKENS__").is_none());
}

#[test]
fn thread_create_marker_rejects_missing_fields() {
    // Missing participants section
    assert!(parse_thread_create("__BPMUX_THREAD_CREATE__|id|name").is_none());
    // Missing name + participants
    assert!(parse_thread_create("__BPMUX_THREAD_CREATE__|id").is_none());
}

// ── #271 thread-close marker parsing ──

use super::event::parse_thread_close;

#[test]
fn thread_close_marker_parses_name_and_by_role() {
    let text = "__BPMUX_THREAD_CLOSE__|cheese|pm";
    let (name, by) = parse_thread_close(text).unwrap();
    assert_eq!(name, "cheese");
    assert_eq!(by, "pm");
}

#[test]
fn thread_close_marker_rejects_non_marker_text() {
    assert!(parse_thread_close("hello world").is_none());
    assert!(parse_thread_close("__BPMUX_THREAD_CREATE__|id|name|p").is_none());
}

#[test]
fn thread_close_marker_rejects_missing_fields() {
    assert!(parse_thread_close("__BPMUX_THREAD_CLOSE__|name").is_none());
    assert!(parse_thread_close("__BPMUX_THREAD_CLOSE__").is_none());
}

// ── #273 suggest-leave marker parsing ──

use super::event::parse_suggest_leave;

#[test]
fn suggest_leave_marker_parses_all_fields() {
    let text = "__BPMUX_SUGGEST_LEAVE__|cheese|dev|pm|focus on #218";
    let (thread, target, caller, reason) = parse_suggest_leave(text).unwrap();
    assert_eq!(thread, "cheese");
    assert_eq!(target, "dev");
    assert_eq!(caller, "pm");
    assert_eq!(reason, "focus on #218");
}

#[test]
fn suggest_leave_marker_preserves_pipes_in_reason() {
    // splitn(4) leaves any trailing pipes in the reason field —
    // thread names and role names are slugified upstream so this
    // only matters for reason, which is free-text.
    let text = "__BPMUX_SUGGEST_LEAVE__|cheese|dev|pm|look at a|b|c";
    let (_, _, _, reason) = parse_suggest_leave(text).unwrap();
    assert_eq!(reason, "look at a|b|c");
}

#[test]
fn suggest_leave_marker_rejects_empty_reason() {
    // Empty reason violates the "context is load-bearing" spec; the
    // bridge enforces non-empty at the tool entry but the parser
    // should reject too as a defense-in-depth check.
    let text = "__BPMUX_SUGGEST_LEAVE__|cheese|dev|pm|";
    assert!(parse_suggest_leave(text).is_none());
}

#[test]
fn suggest_leave_marker_rejects_missing_fields() {
    assert!(parse_suggest_leave("__BPMUX_SUGGEST_LEAVE__|a|b|c").is_none());
    assert!(parse_suggest_leave("__BPMUX_SUGGEST_LEAVE__").is_none());
}

#[test]
fn suggest_leave_marker_rejects_non_marker_text() {
    assert!(parse_suggest_leave("hello world").is_none());
    assert!(parse_suggest_leave("__BPMUX_THREAD_CLOSE__|x|y").is_none());
}

// ── #274 pull-into-thread + join marker parsing ──

use super::event::{parse_pull_into_thread, parse_thread_join};

#[test]
fn pull_into_thread_marker_parses_all_fields() {
    let text = "__BPMUX_PULL_INTO_THREAD__|cheese|designer|dev|need UX lens";
    let (thread, target, caller, reason) = parse_pull_into_thread(text).unwrap();
    assert_eq!(thread, "cheese");
    assert_eq!(target, "designer");
    assert_eq!(caller, "dev");
    assert_eq!(reason, "need UX lens");
}

#[test]
fn pull_into_thread_marker_rejects_empty_reason() {
    assert!(parse_pull_into_thread("__BPMUX_PULL_INTO_THREAD__|t|r|c|").is_none());
}

#[test]
fn pull_into_thread_marker_rejects_cross_marker_text() {
    assert!(parse_pull_into_thread("__BPMUX_SUGGEST_LEAVE__|t|r|c|x").is_none());
}

#[test]
fn thread_join_marker_parses_thread_and_joiner() {
    let text = "__BPMUX_THREAD_JOIN__|cheese|designer";
    let (thread, joiner) = parse_thread_join(text).unwrap();
    assert_eq!(thread, "cheese");
    assert_eq!(joiner, "designer");
}

#[test]
fn thread_join_marker_rejects_missing_fields() {
    assert!(parse_thread_join("__BPMUX_THREAD_JOIN__|cheese").is_none());
    assert!(parse_thread_join("__BPMUX_THREAD_JOIN__").is_none());
}
