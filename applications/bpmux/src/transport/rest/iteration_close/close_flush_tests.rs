use super::notify::iteration_close_flush_prompt;
use crate::message::MessageType;

#[test]
fn the_flush_prompt_broadcasts_to_all_agents_from_the_system() {
    let prompt = iteration_close_flush_prompt();
    assert_eq!(
        prompt.from, "bpmux",
        "the flush prompt is a system signal, not one agent talking"
    );
    assert!(
        prompt.to.is_none(),
        "to should be None for a full broadcast"
    );
    assert_eq!(
        prompt.addressee.as_deref(),
        Some("all"),
        "addressee must be 'all' so every agent treats it as their turn"
    );
    assert_eq!(
        prompt.message_type,
        MessageType::Chat,
        "the prompt reaches agents as ordinary chat they can act on"
    );
}

#[test]
fn the_flush_prompt_text_names_the_marker_and_the_externalize_action() {
    let prompt = iteration_close_flush_prompt();
    assert!(
        prompt.text.contains("[iteration-close]"),
        "text must carry the [iteration-close] marker so agents recognize the signal: got {}",
        prompt.text
    );
    assert!(
        prompt.text.contains("flush unsaved findings"),
        "text must name the externalize action: got {}",
        prompt.text
    );
}
