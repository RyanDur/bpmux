use serde_json::{Map, Value, json};

use super::SubscriptionRegistry;

pub async fn handle_subscribe(
    args: &Map<String, Value>,
    registry: &SubscriptionRegistry,
) -> Result<String, String> {
    let event_class = args
        .get("event_class")
        .and_then(|v| v.as_str())
        .ok_or("missing 'event_class' (string)")?;
    let filter = match args.get("filter") {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(map.clone()),
        Some(_) => return Err("'filter' must be a JSON object".to_string()),
    };
    let subscription_id = registry.subscribe(event_class, filter).await;
    Ok(json!({ "subscription_id": subscription_id, "event_class": event_class }).to_string())
}

pub async fn handle_unsubscribe(
    args: &Map<String, Value>,
    registry: &SubscriptionRegistry,
) -> Result<String, String> {
    let subscription_id = args
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'subscription_id' (string)")?;
    let unsubscribed = registry.unsubscribe(subscription_id).await;
    Ok(json!({ "unsubscribed": unsubscribed }).to_string())
}

pub async fn handle_wait_for_event(
    args: &Map<String, Value>,
    registry: &SubscriptionRegistry,
) -> Result<String, String> {
    let subscription_id = args
        .get("subscription_id")
        .and_then(|v| v.as_str())
        .ok_or("missing 'subscription_id' (string)")?;
    let timeout_ms = args
        .get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(30_000);
    let rx = registry
        .receiver(subscription_id)
        .await
        .ok_or("unknown subscription_id — subscribe first")?;
    let mut rx = rx.lock().await;
    tokio::select! {
        maybe = rx.recv() => match maybe {
            Some(event) => Ok(event.to_string()),
            None => Ok(json!({ "timed_out": true }).to_string()),
        },
        _ = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)) => {
            Ok(json!({ "timed_out": true }).to_string())
        }
    }
}
