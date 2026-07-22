use crate::ws::should_deliver;
use user_routing::short_name;

pub(super) fn matches_any_resolved(
    resolved_ids: &[String],
    matches: impl Fn(Option<&str>) -> bool,
) -> bool {
    if resolved_ids.is_empty() {
        matches(None)
    } else {
        resolved_ids.iter().any(|id| matches(Some(id)))
    }
}

pub(super) fn event_for_role(
    message: &message::Message,
    role: &str,
    resolved_ids: &[String],
    is_owner: bool,
) -> Option<serde_json::Value> {
    if matches_any_resolved(resolved_ids, |id| message.is_from_user(id, role)) {
        return None;
    }
    if let Some(delta) = &message.state_delta {
        let ts = message
            .ts
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
        let (event_class, payload) = delta.to_subscription_event(&ts, &ci::github_actions_url())?;
        return Some(serde_json::json!({
            "event_class": event_class,
            "payload": payload,
            "cursor": ts,
        }));
    }
    let reaches = matches_any_resolved(resolved_ids, |id| {
        should_deliver(message, id, role, is_owner)
    });
    if !message.is_chat() || !reaches {
        return None;
    }
    Some(broadcast_event(message))
}

fn broadcast_event(message: &message::Message) -> serde_json::Value {
    let url_safe_ts = message
        .ts
        .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true);
    serde_json::json!({
        "event_class": "broadcast_received",
        "payload": {
            "from": short_name(&message.from),
            "text": message.text,
            "ts": url_safe_ts,
            "channel": channel_label(&message.to),
        },
        "cursor": url_safe_ts,
    })
}

fn channel_label(to: &Option<Vec<String>>) -> String {
    match to {
        None => "broadcast".to_string(),
        Some(recipients)
            if recipients
                .iter()
                .any(|recipient| recipient == message::BROADCAST_ALL) =>
        {
            "broadcast".to_string()
        }
        Some(recipients) => recipients.join(","),
    }
}
