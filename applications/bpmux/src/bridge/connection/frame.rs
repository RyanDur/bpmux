use serde_json::json;
use tokio::sync::Mutex;

use super::super::{event_subscription, message_type_reaches_llm_stream};
use protocol::ServerMessage;

pub(super) async fn handle_message_frame(
    msg: ServerMessage,
    name: &Mutex<String>,
    user_id: Option<&str>,
    subscriptions: &event_subscription::SubscriptionRegistry,
) {
    let ServerMessage::Message {
        from,
        sender_id,
        text,
        ts,
        to,
        to_ids,
        addressee,
        addressee_ids,
        message_type,
        multi_depth: _,
        state_delta,
        state_snapshot: _,
    } = msg
    else {
        return;
    };
    if state_delta.is_some() {
        subscriptions.advance_cursor(&ts).await;
    }
    if let Some((event, payload)) = state_delta
        .as_deref()
        .and_then(|delta| delta.to_subscription_event(&ts, &crate::ci::github_actions_url()))
    {
        subscriptions.dispatch(event, payload).await;
        return;
    }
    let from_short = from.split(':').next().unwrap_or(&from);
    let current_name = name.lock().await;
    if message::from_matches(&from, sender_id.as_deref(), user_id, &current_name) {
        return;
    }
    drop(current_name);

    if !message_type_reaches_llm_stream(message_type.as_deref()) {
        return;
    }

    let current_name = name.lock().await;
    let relevant = crate::bridge::chat_relevance::scope_reaches_live_stream(
        to.as_deref(),
        to_ids.as_deref(),
        user_id,
        &current_name,
    );
    let channel = match &to {
        None => "broadcast".to_string(),
        Some(recipients) if recipients.iter().any(|r| r == message::BROADCAST_ALL) => {
            "broadcast".to_string()
        }
        Some(recipients) => recipients.join(","),
    };
    drop(current_name);
    if !relevant {
        return;
    }

    let current_name = name.lock().await;
    let addressed_to_me = crate::bridge::chat_relevance::addressed_to_me(
        addressee.as_deref(),
        addressee_ids.as_deref(),
        to.as_deref(),
        to_ids.as_deref(),
        user_id,
        &current_name,
    );
    drop(current_name);
    let sender_is_owner =
        roster::roster_role_for_identity_prefix(from_short) == crate::store::RosterRole::Owner;
    let declared = subscriptions.declared_chat_classes().await;
    if !crate::bridge::chat_relevance::chat_class_reaches_live_stream(
        message_type.as_deref().unwrap_or("chat"),
        addressed_to_me,
        sender_is_owner,
        &declared,
    ) {
        return;
    }

    for class in crate::bridge::chat_relevance::chat_event_classes(&channel, addressee.as_deref()) {
        let mut payload = json!({
            "from": from_short,
            "text": &text,
            "ts": &ts,
            "channel": &channel,
        });
        if class == "addressed_to_me"
            && let Some(a) = addressee.as_deref()
            && let Some(obj) = payload.as_object_mut()
        {
            obj.insert("addressee".to_string(), json!(a));
        }
        subscriptions.dispatch(class, payload).await;
    }
}
