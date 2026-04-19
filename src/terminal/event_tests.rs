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
