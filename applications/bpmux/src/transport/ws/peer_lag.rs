use crate::session;
use crate::session::MessageType;

pub const PEER_LAG_MARKER: &str = "__BPMUX_PEER_LAG__";

pub fn build_peer_lag_message(
    peer: &str,
    count: u64,
    ts: chrono::DateTime<chrono::Utc>,
) -> session::Message {
    let payload = serde_json::json!({ "peer": peer, "count": count });
    session::Message {
        from: "bpmux".to_string(),
        text: format!("{PEER_LAG_MARKER}:{payload}"),
        ts,
        message_type: MessageType::System,
        ..session::Message::default()
    }
}

pub fn parse_peer_lag_payload(text: &str) -> Option<(String, u64)> {
    let rest = text.strip_prefix(PEER_LAG_MARKER)?.strip_prefix(':')?;
    let value: serde_json::Value = serde_json::from_str(rest).ok()?;
    let peer = value.get("peer")?.as_str()?.to_string();
    let count = value.get("count")?.as_u64()?;
    Some((peer, count))
}
