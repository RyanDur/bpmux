use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::transport::protocol::ClientMessage;

/// Marker the bridge emits when an agent calls start_thread (#268).
/// Kept in sync with THREAD_CREATE_MARKER in terminal/event.rs.
/// Format: `__BPMUX_THREAD_CREATE__|<id>|<name>|<p1,p2,...>`
pub(in crate::bridge) const THREAD_CREATE_MARKER: &str = "__BPMUX_THREAD_CREATE__";

/// `start_thread` — create a named thread with a participant list.
/// Emits a system marker to all peers so each TUI can register the
/// thread locally and switch participants' views to the thread.
/// Returns the generated thread id so the caller can tag subsequent
/// send_message calls.
pub(in crate::bridge) async fn handle_start_thread(
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("missing 'name' argument")?;
    let participants: Vec<String> = args
        .get("participants")
        .and_then(|v| v.as_array())
        .ok_or("missing 'participants' argument (array of role names)")?
        .iter()
        .filter_map(|v| v.as_str().map(String::from))
        .collect();
    if participants.is_empty() {
        return Err("participants must be non-empty".into());
    }
    let id = make_thread_id(name);
    let marker = format!(
        "{THREAD_CREATE_MARKER}|{id}|{name}|{}",
        participants.join(",")
    );
    let msg = ClientMessage::Send {
        text: marker,
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        thread_id: None,
    };
    let json = serde_json::to_string(&msg).map_err(|e| format!("serialize: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("start_thread send failed: {e}"))?;

    let response = serde_json::json!({
        "thread_id": id,
        "name": name,
        "participants": participants,
    });
    Ok(response.to_string())
}

/// Derive a stable id from the name + a timestamp suffix so
/// repeat-creations with the same name stay distinguishable. Slug-
/// safe (lowercase, hyphens) so the id is readable in logs.
fn make_thread_id(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let ts = chrono::Utc::now().timestamp_millis();
    if slug.is_empty() {
        format!("thread-{ts}")
    } else {
        format!("{slug}-{ts}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_id_slugifies_name() {
        let id = make_thread_id("let's talk about cheese!");
        assert!(id.starts_with("let-s-talk-about-cheese-"));
    }

    #[test]
    fn thread_id_handles_empty_or_special_name() {
        let id = make_thread_id("!!!");
        assert!(id.starts_with("thread-"));
    }

    #[test]
    fn thread_id_is_unique_per_call() {
        let a = make_thread_id("cheese");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = make_thread_id("cheese");
        assert_ne!(a, b);
    }
}
