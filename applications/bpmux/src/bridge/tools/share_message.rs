use tokio::sync::Mutex;

use super::helpers::{WsSink, urlencoding, ws_send};
use crate::transport::protocol::ClientMessage;

pub(in crate::bridge) fn parse_share_scope(scope: &str) -> Result<Option<Vec<String>>, String> {
    if scope.is_empty() {
        return Err("empty scope".to_string());
    }
    if scope == crate::session::message::BROADCAST_ALL {
        return Ok(None);
    }
    let recipients: Vec<String> = scope
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if recipients.is_empty() {
        return Err("empty scope".to_string());
    }
    Ok(Some(recipients))
}

pub(in crate::bridge) fn format_share_marker(
    agent: &str,
    source_scope: Option<&str>,
    text: &str,
) -> String {
    match source_scope {
        Some(s) => format!("[Shared by {agent} from {s} thread] {text}"),
        None => format!("[Shared by {agent}] {text}"),
    }
}

pub(in crate::bridge) fn pick_most_recent_visible_chat(
    messages: &[serde_json::Value],
) -> Option<&serde_json::Value> {
    messages
        .iter()
        .find(|m| m.get("message_type").map(|v| v.is_null()).unwrap_or(true))
}

pub(in crate::bridge) fn build_reshare_message(
    source: &serde_json::Value,
    agent_short: &str,
    target_to: Option<Vec<String>>,
) -> Result<ClientMessage, String> {
    if source
        .get("explicit_privacy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err(
            "cannot share a private message; paraphrase it in your own words if it needs wider visibility"
                .to_string(),
        );
    }
    let source_text = source
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("source message has no text")?;
    let source_to = source.get("to").and_then(|v| v.as_array());
    let source_scope = match source_to {
        None => "broadcast".to_string(),
        Some(recipients) if recipients.is_empty() => "broadcast".to_string(),
        Some(_) => {
            let sender_short = source
                .get("from")
                .and_then(|v| v.as_str())
                .and_then(|s| s.split(':').next())
                .unwrap_or("peer");
            sender_short.to_string()
        }
    };
    let marker = format_share_marker(agent_short, Some(&source_scope), source_text);
    Ok(ClientMessage::Send {
        text: marker,
        to: target_to,
        addressee: None,
        message_type: None,
    })
}

pub(in crate::bridge) async fn handle_share_message(
    args: &serde_json::Map<String, serde_json::Value>,
    port: u16,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    let scope_str = args
        .get("scope")
        .and_then(|v| v.as_str())
        .ok_or("missing 'scope' argument (use \"all\" or a comma-list of roles)")?;
    let target_to =
        parse_share_scope(scope_str).map_err(|_| format!("invalid scope: {scope_str:?}"))?;
    let fresh_text = args
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let short = agent_name.split(':').next().unwrap_or(agent_name);

    if let Some(body) = fresh_text {
        let marker = format_share_marker(short, None, body);
        let json = serde_json::to_string(&ClientMessage::Send {
            text: marker,
            to: target_to.clone(),
            addressee: None,
            message_type: None,
        })
        .map_err(|e| format!("failed to serialize: {e}"))?;
        ws_send(ws_tx, json).await?;
        return Ok(format!(
            "shared fresh message to {}",
            scope_str_or_all(scope_str)
        ));
    }

    let as_encoded = urlencoding(short);
    let url = crate::transport::local_url::api_url(
        port,
        &format!("/api/messages?limit=20&as={as_encoded}"),
    );
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch log: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read log: {e}"))?;
    let messages: Vec<serde_json::Value> =
        serde_json::from_str(&body).map_err(|e| format!("failed to parse log: {e}"))?;
    let Some(source) = pick_most_recent_visible_chat(&messages) else {
        return Err("No recent visible chat to share.".to_string());
    };

    let json = serde_json::to_string(&build_reshare_message(source, short, target_to)?)
        .map_err(|e| format!("failed to serialize: {e}"))?;
    ws_send(ws_tx, json).await?;
    Ok(format!(
        "shared recent message to {}",
        scope_str_or_all(scope_str)
    ))
}

fn scope_str_or_all(scope: &str) -> &str {
    if scope == crate::session::message::BROADCAST_ALL {
        "all"
    } else {
        scope
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn private_source() -> serde_json::Value {
        serde_json::json!({
            "from": "designer:claude-code",
            "text": "confidential design note",
            "explicit_privacy": true,
            "to": ["dev"]
        })
    }

    fn shareable_broadcast_source() -> serde_json::Value {
        serde_json::json!({
            "from": "pm:claude-code",
            "text": "open broadcast note",
            "to": serde_json::Value::Null
        })
    }

    #[test]
    fn reshare_refuses_explicit_privacy_source_and_does_not_widen() {
        let err = build_reshare_message(&private_source(), "pm", Some(vec!["all".to_string()]))
            .expect_err("an explicit_privacy source must be refused");
        assert!(
            err.contains("cannot share a private message"),
            "refusal names the privacy cause; got {err}"
        );
        assert!(
            build_reshare_message(&private_source(), "pm", None).is_err(),
            "a private source must never produce a ClientMessage::Send — no widening to any scope"
        );
    }

    #[test]
    fn reshare_drops_addressee_on_propagated_message() {
        match build_reshare_message(
            &shareable_broadcast_source(),
            "dev",
            Some(vec!["all".into()]),
        )
        .expect("non-private source builds a Send")
        {
            ClientMessage::Send { addressee, .. } => assert_eq!(
                addressee, None,
                "propagated message must not inherit the source addressee"
            ),
            other => panic!("expected ClientMessage::Send, got {other:?}"),
        }
    }

    #[test]
    fn reshare_marks_source_scope_broadcast_when_source_has_no_recipients() {
        match build_reshare_message(&shareable_broadcast_source(), "dev", None).unwrap() {
            ClientMessage::Send { text, .. } => assert!(
                text.contains("from broadcast thread"),
                "no-recipient source marks broadcast provenance; got {text}"
            ),
            other => panic!("expected Send, got {other:?}"),
        }
    }

    #[test]
    fn reshare_marks_source_scope_counterpart_for_dm() {
        let dm = serde_json::json!({"from": "designer:claude-code", "text": "x", "to": ["dev"]});
        match build_reshare_message(&dm, "dev", None).unwrap() {
            ClientMessage::Send { text, .. } => assert!(
                text.contains("from designer thread"),
                "DM source marks the counterpart as provenance scope; got {text}"
            ),
            other => panic!("expected Send, got {other:?}"),
        }
    }

    #[test]
    fn format_share_marker_form1_includes_source_scope() {
        assert_eq!(
            format_share_marker("dev", Some("pm"), "body"),
            "[Shared by dev from pm thread] body"
        );
    }

    #[test]
    fn format_share_marker_form2_omits_source_scope() {
        assert_eq!(
            format_share_marker("dev", None, "body"),
            "[Shared by dev] body"
        );
    }

    #[test]
    fn parse_share_scope_all_widens_to_broadcast() {
        assert_eq!(
            parse_share_scope(crate::session::message::BROADCAST_ALL),
            Ok(None)
        );
    }

    #[test]
    fn parse_share_scope_comma_list_splits_into_recipients() {
        assert_eq!(
            parse_share_scope("pm, designer"),
            Ok(Some(vec!["pm".to_string(), "designer".to_string()]))
        );
    }

    #[test]
    fn parse_share_scope_empty_or_whitespace_is_err() {
        assert!(parse_share_scope("").is_err());
        assert!(parse_share_scope("  ,  ").is_err());
    }

    #[test]
    fn pick_most_recent_skips_non_chat_then_returns_chat() {
        let messages = vec![
            serde_json::json!({"message_type": "activity", "text": "act"}),
            serde_json::json!({"message_type": serde_json::Value::Null, "text": "real chat"}),
        ];
        assert_eq!(
            pick_most_recent_visible_chat(&messages).unwrap()["text"],
            "real chat"
        );
    }

    #[test]
    fn pick_most_recent_treats_missing_message_type_as_chat() {
        let messages = vec![serde_json::json!({"text": "untyped chat"})];
        assert!(pick_most_recent_visible_chat(&messages).is_some());
    }

    #[test]
    fn pick_most_recent_none_when_all_non_chat() {
        let messages = vec![serde_json::json!({"message_type": "system", "text": "sys"})];
        assert!(pick_most_recent_visible_chat(&messages).is_none());
    }
}
