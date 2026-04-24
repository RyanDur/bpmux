use serde::Serialize;
use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::session::diag::log_diag;
use crate::transport::protocol::ClientMessage;

const OWNER_ROLE: &str = "ryan";

#[derive(Serialize)]
#[serde(untagged)]
enum DeliveredTo {
    Broadcast(&'static str),
    Scoped(Vec<String>),
}

#[derive(Serialize)]
struct DeliveryEcho {
    status: &'static str,
    delivered_to: DeliveredTo,
    visible_to_owner: bool,
}

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

fn delivery_echo(to: Option<&[String]>) -> DeliveryEcho {
    match to {
        None => DeliveryEcho {
            status: "sent",
            delivered_to: DeliveredTo::Broadcast("broadcast"),
            visible_to_owner: true,
        },
        Some(list) if list.is_empty() || list.iter().any(|r| r == "all") => DeliveryEcho {
            status: "sent",
            delivered_to: DeliveredTo::Broadcast("broadcast"),
            visible_to_owner: true,
        },
        Some(list) => {
            let visible_to_owner = list.iter().any(|r| r == OWNER_ROLE);
            DeliveryEcho {
                status: "sent",
                delivered_to: DeliveredTo::Scoped(list.to_vec()),
                visible_to_owner,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_when_to_is_none() {
        let echo = delivery_echo(None);
        assert!(echo.visible_to_owner);
        assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
    }

    #[test]
    fn broadcast_when_to_contains_all() {
        let to = vec!["all".to_string()];
        let echo = delivery_echo(Some(&to));
        assert!(echo.visible_to_owner);
        assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
    }

    #[test]
    fn scoped_excludes_owner_when_owner_not_in_to() -> Result<(), serde_json::Error> {
        let to = vec!["dev".to_string(), "pm".to_string()];
        let echo = delivery_echo(Some(&to));
        assert!(!echo.visible_to_owner);
        let serialized = serde_json::to_value(&echo)?;
        assert_eq!(serialized["delivered_to"], serde_json::json!(["dev", "pm"]));
        Ok(())
    }

    #[test]
    fn broadcast_serializes_as_string() -> Result<(), serde_json::Error> {
        let echo = delivery_echo(None);
        let serialized = serde_json::to_value(&echo)?;
        assert_eq!(serialized["delivered_to"], serde_json::json!("broadcast"));
        assert_eq!(serialized["visible_to_owner"], serde_json::json!(true));
        assert_eq!(serialized["status"], serde_json::json!("sent"));
        Ok(())
    }

    #[test]
    fn scoped_includes_owner_when_owner_in_to() {
        let to = vec!["ryan".to_string(), "dev".to_string()];
        let echo = delivery_echo(Some(&to));
        assert!(echo.visible_to_owner);
    }
}
