pub(crate) const CHAT_RELEVANCE_CLASSES: [&str; 3] = ["chat", "insight", "nudge"];

pub(crate) fn chat_class_reaches_live_stream(
    message_class: &str,
    addressed_to_me: bool,
    sender_is_owner: bool,
    declared_chat_classes: &[String],
) -> bool {
    if addressed_to_me || sender_is_owner {
        return true;
    }
    if declared_chat_classes.is_empty() {
        return message_class != "nudge";
    }
    declared_chat_classes.iter().any(|c| c == message_class)
}

#[cfg(test)]
mod tests {
    use super::chat_class_reaches_live_stream;

    fn classes(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    #[test]
    fn addressed_to_me_is_un_muteable_even_when_class_off() {
        assert!(
            chat_class_reaches_live_stream("nudge", true, false, &classes(&["chat"])),
            "#1648/1a: addressed-to-me is the floor — pushes regardless of relevance-set or class"
        );
    }

    #[test]
    fn owner_sender_is_un_muteable_even_with_empty_set() {
        assert!(
            chat_class_reaches_live_stream("nudge", false, true, &[]),
            "#1648/1a: owner-sent is the floor — un-muteable even under the curated default"
        );
    }

    #[test]
    fn curated_default_pushes_broadcast_chat() {
        assert!(
            chat_class_reaches_live_stream("chat", false, false, &[]),
            "#1648/1a: empty set => curated default keeps broadcast-chat ON (team awareness)"
        );
    }

    #[test]
    fn curated_default_pushes_insight() {
        assert!(
            chat_class_reaches_live_stream("insight", false, false, &[]),
            "#1648/1a: insight not in the OFF-set (activity/system/others-nudges) => default ON"
        );
    }

    #[test]
    fn curated_default_suppresses_non_addressed_nudge() {
        assert!(
            !chat_class_reaches_live_stream("nudge", false, false, &[]),
            "#1648/1a: empty set => curated default suppresses others'-nudges from the live-push"
        );
    }

    #[test]
    fn custom_set_pushes_in_set_class() {
        assert!(
            chat_class_reaches_live_stream("chat", false, false, &classes(&["chat", "insight"])),
            "#1648/1a: declared relevance-set => in-set class is pushed"
        );
    }

    #[test]
    fn custom_set_suppresses_out_of_set_class_for_same_agent() {
        assert!(
            !chat_class_reaches_live_stream("insight", false, false, &classes(&["chat"])),
            "#1648/1a: SAME agent, declared chat-only => insight (out-of-set) suppressed — non-vacuous discrimination"
        );
    }

    #[test]
    fn custom_set_suppresses_broadcast_chat_when_muted() {
        assert!(
            !chat_class_reaches_live_stream("chat", false, false, &classes(&["insight"])),
            "#1648/1a: an agent can MUTE broadcast-chat by declaring a set that omits it"
        );
    }

    #[test]
    fn floor_wins_over_custom_set_omission() {
        assert!(
            chat_class_reaches_live_stream("chat", true, false, &classes(&["insight"])),
            "#1648/1a: addressed-to-me floor overrides a relevance-set that omits the class"
        );
    }
}
