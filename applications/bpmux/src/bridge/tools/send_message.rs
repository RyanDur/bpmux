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
enum DeliveredTo {
    Broadcast(&'static str),
    Scoped(Vec<String>),
}

#[derive(Serialize)]
struct DeliveryEcho {
    status: &'static str,
    delivered_to: DeliveredTo,
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

fn delivery_echo(to: Option<&[String]>) -> DeliveryEcho {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_when_to_is_none() {
        let echo = delivery_echo(None);
        assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
    }

    #[test]
    fn broadcast_when_to_contains_all() {
        let to = vec!["all".to_string()];
        let echo = delivery_echo(Some(&to));
        assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
    }

    #[test]
    fn owner_auto_included_when_absent_from_to() {
        let to = Some(vec!["dev".to_string(), "pm".to_string()]);
        let adjusted = ensure_owner_included(to).expect("scoped");
        assert!(adjusted.iter().any(|r| r == "ryan"));
    }

    #[test]
    fn owner_not_duplicated_when_already_in_to() {
        let to = Some(vec!["ryan".to_string(), "dev".to_string()]);
        let adjusted = ensure_owner_included(to).expect("scoped");
        assert_eq!(adjusted.iter().filter(|r| **r == "ryan").count(), 1);
    }

    #[test]
    fn broadcast_passes_through_unchanged() {
        assert!(ensure_owner_included(None).is_none());
    }

    #[test]
    fn all_keyword_passes_through_unchanged() {
        let to = Some(vec!["all".to_string()]);
        let adjusted = ensure_owner_included(to).expect("scoped");
        assert_eq!(adjusted, vec!["all".to_string()]);
    }

    #[test]
    fn empty_scope_passes_through_unchanged() {
        let to = Some(vec![]);
        let adjusted = ensure_owner_included(to).expect("scoped");
        assert!(adjusted.is_empty());
    }

    #[test]
    fn broadcast_serializes_as_string() -> Result<(), serde_json::Error> {
        let echo = delivery_echo(None);
        let serialized = serde_json::to_value(&echo)?;
        assert_eq!(serialized["delivered_to"], serde_json::json!("broadcast"));
        assert_eq!(serialized["status"], serde_json::json!("sent"));
        Ok(())
    }

    #[test]
    fn echo_has_no_visible_to_owner_field() -> Result<(), serde_json::Error> {
        let echo = delivery_echo(Some(&["dev".to_string()]));
        let serialized = serde_json::to_value(&echo)?;
        assert!(
            serialized.get("visible_to_owner").is_none(),
            "visible_to_owner field must be absent: {serialized}"
        );
        Ok(())
    }

    #[test]
    fn scoped_echo_surfaces_list_with_owner_auto_added() -> Result<(), serde_json::Error> {
        let adjusted = ensure_owner_included(Some(vec!["dev".to_string()])).expect("scoped");
        let echo = delivery_echo(Some(&adjusted));
        let serialized = serde_json::to_value(&echo)?;
        let list = serialized["delivered_to"]
            .as_array()
            .expect("array")
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();
        assert!(list.contains(&"dev"));
        assert!(list.contains(&"ryan"));
        Ok(())
    }
}
