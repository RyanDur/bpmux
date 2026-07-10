use super::chat_relevance::chat_event_classes;

#[test]
fn broadcast_chat_dispatches_broadcast_received() {
    assert_eq!(
        chat_event_classes("broadcast", None),
        vec!["broadcast_received"]
    );
}

#[test]
fn directed_addressee_dispatches_addressed_to_me() {
    assert_eq!(
        chat_event_classes("pm,dev", Some("dev")),
        vec!["addressed_to_me"]
    );
}

#[test]
fn broadcast_with_addressee_dispatches_both() {
    assert_eq!(
        chat_event_classes("broadcast", Some("dev")),
        vec!["broadcast_received", "addressed_to_me"]
    );
}

#[test]
fn broadcast_all_addressee_is_not_a_directed_addressee() {
    assert_eq!(
        chat_event_classes("broadcast", Some("all")),
        vec!["broadcast_received"]
    );
}

#[test]
fn scoped_chat_with_no_addressee_dispatches_nothing_no_leak() {
    assert!(chat_event_classes("pm,dev", None).is_empty());
}
