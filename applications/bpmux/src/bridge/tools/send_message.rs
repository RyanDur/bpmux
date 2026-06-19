use serde::Serialize;
use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::session::diag::log_diag;
use crate::transport::protocol::ClientMessage;

fn owner_role_id() -> String {
    crate::workflow::current_workflow()
        .owner_role_id()
        .map(str::to_string)
        .unwrap_or_else(|| "ryan".to_string())
}

#[derive(Serialize)]
#[serde(untagged)]
pub(super) enum DeliveredTo {
    Broadcast(&'static str),
    Scoped(Vec<String>),
}

#[derive(Serialize)]
pub(super) struct DeliveryEcho {
    status: &'static str,
    pub(super) delivered_to: DeliveredTo,
}

pub fn ensure_owner_included(to: Option<Vec<String>>) -> Option<Vec<String>> {
    let mut list = to?;
    if list.is_empty() || list.iter().any(|r| r == "all") {
        return Some(list);
    }
    let owner = owner_role_id();
    if !list.iter().any(|r| r == &owner) {
        list.push(owner);
    }
    Some(list)
}

pub(in crate::bridge) async fn handle_send(
    args: &serde_json::Map<String, serde_json::Value>,
    sender_role: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("missing 'text' argument")?;
    if let Some(reason) = crate::session::speech_guard::pause_vocab_violation(text) {
        return Err(reason);
    }
    let explicit_to = args.get("to").and_then(|v| v.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect::<Vec<String>>()
    });
    let to = ensure_owner_included(explicit_to);
    let addressee = args
        .get("addressee")
        .and_then(|v| v.as_str())
        .map(String::from);
    let message_type = args
        .get("message_type")
        .and_then(|v| v.as_str())
        .map(String::from);
    let recipient_desc = match &to {
        Some(list) => format!("{list:?}"),
        None => "broadcast".to_string(),
    };
    let addressee_desc = addressee.as_deref().unwrap_or("-");
    log_diag(&format!(
        "SEND status=pending sender={sender_role} to={recipient_desc} \
         addressee={addressee_desc} size={}",
        text.len()
    ));
    let json = serde_json::to_string(&ClientMessage::Send {
        text: text.to_string(),
        to: to.clone(),
        addressee,
        message_type,
    })
    .map_err(|e| format!("failed to serialize: {e}"))?;
    match ws_send(ws_tx, json).await {
        Ok(()) => {
            log_diag(&format!(
                "SEND status=acked sender={sender_role} size={}",
                text.len()
            ));
            let echo = delivery_echo(to.as_deref());
            serde_json::to_string(&echo).map_err(|e| format!("failed to serialize echo: {e}"))
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

pub(super) fn delivery_echo(to: Option<&[String]>) -> DeliveryEcho {
    match to {
        None => DeliveryEcho {
            status: "sent",
            delivered_to: DeliveredTo::Broadcast("broadcast"),
        },
        Some(list) if list.is_empty() || list.iter().any(|r| r == "all") => DeliveryEcho {
            status: "sent",
            delivered_to: DeliveredTo::Broadcast("broadcast"),
        },
        Some(list) => DeliveryEcho {
            status: "sent",
            delivered_to: DeliveredTo::Scoped(list.to_vec()),
        },
    }
}
