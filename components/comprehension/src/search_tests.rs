#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::search::{SearchFilters, evaluate_candidate};
use chrono::Utc;
use message::{Message, MessageType};

fn chat_to(text: &str, to: Option<Vec<String>>, to_ids: Option<Vec<String>>) -> Message {
    Message {
        from: "dev:claude-code".to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to,
        to_ids,
        ..Message::default()
    }
}

fn addressed_filter(addressee: &str) -> SearchFilters {
    SearchFilters {
        query: "needle".to_string(),
        sender: None,
        after: None,
        before: None,
        channel: None,
        addressee: Some(addressee.to_string()),
        message_class: None,
        insight_only: false,
        owner_name: "owner".to_string(),
    }
}

#[test]
fn a_message_addressed_to_the_role_matches() {
    let cluster = vec![chat_to("the needle", Some(vec!["pm".to_string()]), None)];
    let result = evaluate_candidate(&cluster, 0, &addressed_filter("pm"));
    assert!(result.is_some(), "role addressing satisfies the filter");
}

#[test]
fn a_message_addressed_to_the_user_id_matches() {
    let cluster = vec![chat_to(
        "the needle",
        None,
        Some(vec!["6b3054e9-a6b4-4e5b-84db-f788ebdec25b".to_string()]),
    )];
    let result = evaluate_candidate(
        &cluster,
        0,
        &addressed_filter("6b3054e9-a6b4-4e5b-84db-f788ebdec25b"),
    );
    assert!(result.is_some(), "id addressing satisfies the filter");
}

#[test]
fn a_broadcast_is_not_addressed_to_anyone() {
    let cluster = vec![chat_to("the needle", None, None)];
    let result = evaluate_candidate(&cluster, 0, &addressed_filter("pm"));
    assert!(
        result.is_none(),
        "broadcasts reach everyone but are addressed to no one"
    );
}

#[test]
fn addressing_someone_else_does_not_match() {
    let cluster = vec![chat_to(
        "the needle",
        Some(vec!["designer".to_string()]),
        None,
    )];
    let result = evaluate_candidate(&cluster, 0, &addressed_filter("pm"));
    assert!(result.is_none());
}
