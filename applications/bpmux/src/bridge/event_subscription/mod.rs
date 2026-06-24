use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::{Map, Value, json};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

mod handlers;
mod persistence;

#[cfg(test)]
mod tests;

pub(in crate::bridge) use handlers::{handle_subscribe, handle_unsubscribe, handle_wait_for_event};
pub(crate) use persistence::subscriptions_path;
use persistence::{
    PersistedSubscription, cursor_path, load_cursor, load_persisted, write_cursor, write_persisted,
};

struct Subscription {
    event_class: String,
    filter: Option<Map<String, Value>>,
    tx: UnboundedSender<Value>,
    rx: Arc<Mutex<UnboundedReceiver<Value>>>,
}

fn filter_matches(filter: &Option<Map<String, Value>>, payload: &Value) -> bool {
    crate::message::payload_matches_filter(filter.as_ref(), payload)
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

    pub(super) async fn subscribe(
        &self,
        event_class: &str,
        filter: Option<Map<String, Value>>,
    ) -> String {
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

    pub(super) async fn unsubscribe(&self, subscription_id: &str) -> bool {
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

    pub(super) async fn receiver(
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

    pub(crate) async fn declared_chat_classes(&self) -> Vec<String> {
        let subs = self.subs.lock().await;
        subs.values()
            .map(|s| s.event_class.clone())
            .filter(|c| crate::bridge::chat_relevance::CHAT_RELEVANCE_CLASSES.contains(&c.as_str()))
            .collect()
    }
}
