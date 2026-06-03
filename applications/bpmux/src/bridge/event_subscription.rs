use std::collections::HashMap;
use std::sync::Arc;

use serde_json::{Map, Value, json};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

struct Subscription {
    event_class: String,
    tx: UnboundedSender<Value>,
    rx: Arc<Mutex<UnboundedReceiver<Value>>>,
}

#[derive(Clone, Default)]
pub(crate) struct SubscriptionRegistry {
    subs: Arc<Mutex<HashMap<String, Subscription>>>,
}

impl SubscriptionRegistry {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    async fn subscribe(&self, event_class: &str) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = unbounded_channel();
        self.subs.lock().await.insert(
            id.clone(),
            Subscription {
                event_class: event_class.to_string(),
                tx,
                rx: Arc::new(Mutex::new(rx)),
            },
        );
        id
    }

    async fn unsubscribe(&self, subscription_id: &str) -> bool {
        self.subs.lock().await.remove(subscription_id).is_some()
    }

    async fn receiver(
        &self,
        subscription_id: &str,
    ) -> Option<Arc<Mutex<UnboundedReceiver<Value>>>> {
        self.subs
            .lock()
            .await
            .get(subscription_id)
            .map(|s| s.rx.clone())
    }

    pub(crate) async fn dispatch(&self, event_class: &str, payload: Value) {
        let event = json!({ "event_class": event_class, "payload": payload });
        let subs = self.subs.lock().await;
        for sub in subs.values() {
            if sub.event_class == event_class {
                let _ = sub.tx.send(event.clone());
            }
        }
    }
}

pub(in crate::bridge) async fn handle_subscribe(
    args: &Map<String, Value>,
    registry: &SubscriptionRegistry,
) -> Result<String, String> {
    let event_class = args
        .get("event_class")
        .and_then(|v| v.as_str())
        .ok_or("missing 'event_class' (string)")?;
    let subscription_id = registry.subscribe(event_class).await;
    Ok(json!({ "subscription_id": subscription_id, "event_class": event_class }).to_string())
}

pub(in crate::bridge) async fn handle_unsubscribe(
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

pub(in crate::bridge) async fn handle_wait_for_event(
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
