use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::session::diag::log_diag;
use crate::transport::protocol::ClientMessage;

pub(in crate::bridge) async fn handle_send(
    args: &serde_json::Map<String, serde_json::Value>,
    sender_role: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    last_channel: &Mutex<Option<Vec<String>>>,
) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("missing 'text' argument")?;
    let explicit_to = args.get("to").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>()
    });
    let to = match explicit_to {
        Some(t) => Some(t),
        None => last_channel.lock().await.clone(),
    };
    // Attention signal (#158) — optional `addressee` arg identifies the
    // role whose turn it is to respond. Agents without that role see the
    // message but should skip responding. See docs/turn-aware-channel-spec.md.
    let addressee = args
        .get("addressee")
        .and_then(|v| v.as_str())
        .map(String::from);
    let message_type = args
        .get("message_type")
        .and_then(|v| v.as_str())
        .map(String::from);
    let thread_id = args
        .get("thread_id")
        .and_then(|v| v.as_str())
        .map(String::from);
    // Audit trail (#167): log pending BEFORE the ws_send so a drop
    // at any layer below leaves a durable "this was attempted" record
    // in diag. Paired with a status=acked / status=failed line after,
    // observers can correlate attempts with outcomes without needing
    // console output from the sender.
    let recipient_desc = match &to {
        Some(list) => format!("{list:?}"),
        None => "broadcast".to_string(),
    };
    let addressee_desc = addressee.as_deref().unwrap_or("-");
    let thread_desc = thread_id.as_deref().unwrap_or("-");
    log_diag(&format!(
        "SEND status=pending sender={sender_role} to={recipient_desc} \
         addressee={addressee_desc} thread={thread_desc} size={}",
        text.len()
    ));
    let json = serde_json::to_string(&ClientMessage::Send {
        text: text.to_string(),
        to,
        addressee,
        message_type,
        thread_id,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    match ws_send(ws_tx, json).await {
        Ok(()) => {
            log_diag(&format!(
                "SEND status=acked sender={sender_role} size={}",
                text.len()
            ));
            Ok("message sent".into())
        }
        Err(e) => {
            log_diag(&format!(
                "SEND status=failed sender={sender_role} size={} error={e}",
                text.len()
            ));
            Err(e)
        }
    }
}
