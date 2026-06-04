use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

struct Subscription {
    event_class: String,
    filter: Option<Map<String, Value>>,
    tx: UnboundedSender<Value>,
    rx: Arc<Mutex<UnboundedReceiver<Value>>>,
}

#[derive(Serialize, Deserialize)]
struct PersistedSubscription {
    id: String,
    event_class: String,
    filter: Option<Map<String, Value>>,
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn subscriptions_path(name: &str) -> Option<PathBuf> {
    let dir = crate::session::log::default_sessions_base()
        .ok()?
        .join("bridge-subscriptions");
    Some(dir.join(format!("{}.json", sanitize_name(name))))
}

fn cursor_path(name: &str) -> Option<PathBuf> {
    let dir = crate::session::log::default_sessions_base()
        .ok()?
        .join("bridge-subscriptions");
    Some(dir.join(format!("{}.cursor", sanitize_name(name))))
}

fn load_cursor(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn write_cursor(path: &Path, cursor: &str) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let tmp = path.with_extension("cursor.tmp");
    if std::fs::write(&tmp, cursor).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn load_persisted(path: &Path) -> Vec<PersistedSubscription> {
    match std::fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn write_persisted(path: &Path, records: &[PersistedSubscription]) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(body) = serde_json::to_string_pretty(records) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn filter_matches(filter: &Option<Map<String, Value>>, payload: &Value) -> bool {
    match filter {
        None => true,
        Some(criteria) => criteria
            .iter()
            .all(|(key, expected)| payload.get(key) == Some(expected)),
    }
}

fn snapshot(subs: &HashMap<String, Subscription>) -> Vec<PersistedSubscription> {
    subs.iter()
        .map(|(id, sub)| PersistedSubscription {
            id: id.clone(),
            event_class: sub.event_class.clone(),
            filter: sub.filter.clone(),
        })
        .collect()
}

#[derive(Clone, Default)]
pub(crate) struct SubscriptionRegistry {
    subs: Arc<Mutex<HashMap<String, Subscription>>>,
    persist_path: Option<PathBuf>,
    cursor: Arc<Mutex<Option<String>>>,
    cursor_path: Option<PathBuf>,
}

impl SubscriptionRegistry {
    pub(crate) fn with_persistence(name: &str) -> Self {
        let subs_path = subscriptions_path(name);
        let cursor_path = cursor_path(name);
        let mut map = HashMap::new();
        if let Some(path) = &subs_path {
            for persisted in load_persisted(path) {
                let (tx, rx) = unbounded_channel();
                map.insert(
                    persisted.id,
                    Subscription {
                        event_class: persisted.event_class,
                        filter: persisted.filter,
                        tx,
                        rx: Arc::new(Mutex::new(rx)),
                    },
                );
            }
        }
        let cursor = cursor_path.as_ref().and_then(|p| load_cursor(p));
        Self {
            subs: Arc::new(Mutex::new(map)),
            persist_path: subs_path,
            cursor: Arc::new(Mutex::new(cursor)),
            cursor_path,
        }
    }

    pub(crate) async fn cursor(&self) -> Option<String> {
        self.cursor.lock().await.clone()
    }

    pub(crate) async fn advance_cursor(&self, ts: &str) {
        {
            let mut cursor = self.cursor.lock().await;
            *cursor = Some(ts.to_string());
        }
        if let Some(path) = &self.cursor_path {
            write_cursor(path, ts);
        }
    }

    async fn init_cursor_floor(&self) {
        let mut cursor = self.cursor.lock().await;
        if cursor.is_none() {
            let now = chrono::Utc::now().to_rfc3339();
            if let Some(path) = &self.cursor_path {
                write_cursor(path, &now);
            }
            *cursor = Some(now);
        }
    }

    async fn subscribe(&self, event_class: &str, filter: Option<Map<String, Value>>) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = unbounded_channel();
        let records = {
            let mut subs = self.subs.lock().await;
            subs.insert(
                id.clone(),
                Subscription {
                    event_class: event_class.to_string(),
                    filter,
                    tx,
                    rx: Arc::new(Mutex::new(rx)),
                },
            );
            self.persist_path.is_some().then(|| snapshot(&subs))
        };
        if let (Some(path), Some(records)) = (&self.persist_path, records) {
            write_persisted(path, &records);
        }
        self.init_cursor_floor().await;
        id
    }

    async fn unsubscribe(&self, subscription_id: &str) -> bool {
        let records = {
            let mut subs = self.subs.lock().await;
            if subs.remove(subscription_id).is_none() {
                return false;
            }
            self.persist_path.is_some().then(|| snapshot(&subs))
        };
        if let (Some(path), Some(records)) = (&self.persist_path, records) {
            write_persisted(path, &records);
        }
        true
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
            if sub.event_class == event_class && filter_matches(&sub.filter, &payload) {
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
    let filter = match args.get("filter") {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) => Some(map.clone()),
        Some(_) => return Err("'filter' must be a JSON object".to_string()),
    };
    let subscription_id = registry.subscribe(event_class, filter).await;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn criteria(pairs: &[(&str, Value)]) -> Option<Map<String, Value>> {
        let mut map = Map::new();
        for (key, value) in pairs {
            map.insert((*key).to_string(), value.clone());
        }
        Some(map)
    }

    #[test]
    fn none_filter_matches_any_payload() {
        assert!(filter_matches(
            &None,
            &json!({ "head_sha": "A", "run_id": 42 })
        ));
        assert!(filter_matches(&None, &json!({})));
    }

    #[test]
    fn submap_filter_matches_payload_superset() {
        let filter = criteria(&[("head_sha", json!("A"))]);
        assert!(filter_matches(
            &filter,
            &json!({ "head_sha": "A", "run_id": 42 })
        ));
    }

    #[test]
    fn filter_does_not_match_on_value_mismatch() {
        let filter = criteria(&[("head_sha", json!("A"))]);
        assert!(!filter_matches(&filter, &json!({ "head_sha": "B" })));
    }

    #[test]
    fn filter_on_absent_key_does_not_match() {
        let filter = criteria(&[("head_sha", json!("A"))]);
        assert!(!filter_matches(&filter, &json!({ "run_id": 42 })));
    }
}
