use std::sync::Arc;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use base64::Engine;
use futures::stream::SplitSink;

use super::client::ClientCtx;
use message::MessageType;

pub(super) fn handle_send(
    ctx: &ClientCtx<'_>,
    identity: &str,
    msg_text: String,
    to: Option<Vec<String>>,
    addressee: Option<String>,
    message_type: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let explicit_privacy = to.is_some();
    let mut msg = message::Message {
        from: identity.to_string(),
        sender_id: ctx.routing_user_id.clone(),
        text: msg_text,
        ts: chrono::Utc::now(),
        message_type: MessageType::from(message_type.as_deref()),
        to,
        addressee,
        explicit_privacy,
        ..message::Message::default()
    };
    crate::transport::delivery_ids::stamp_delivery_ids(&ctx.store.selector(), &mut msg);
    ctx.hub.broadcast(&msg)?;
    Ok(())
}

pub(super) fn handle_pty_subscribe(
    ctx: &ClientCtx<'_>,
    terminal_id: String,
    pty_readers: &mut std::collections::HashMap<String, Arc<std::sync::atomic::AtomicBool>>,
) {
    let already_streaming = pty_readers
        .get(&terminal_id)
        .is_some_and(|stop| !stop.load(std::sync::atomic::Ordering::Relaxed));
    if !already_streaming && let Some(mut output) = ctx.pty_registry.subscribe_for(&terminal_id) {
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        pty_readers.insert(terminal_id.clone(), stop.clone());
        let sink = ctx.pty_tx.clone();
        tokio::spawn(async move {
            use tokio::sync::broadcast::error::RecvError;
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                match output.recv().await {
                    Ok(bytes) if bytes.is_empty() => {
                        let _ = sink.send((terminal_id.clone(), Vec::new()));
                        break;
                    }
                    Ok(bytes) => {
                        if sink.send((terminal_id.clone(), bytes)).is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(_)) => continue,
                    Err(RecvError::Closed) => {
                        let _ = sink.send((terminal_id.clone(), Vec::new()));
                        break;
                    }
                }
            }
        });
    }
}

pub(super) fn handle_pty_input(ctx: &ClientCtx<'_>, terminal_id: String, data: String) {
    if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(&data) {
        ctx.pty_registry.note_human_keystroke(&terminal_id);
        ctx.pty_registry.write_to(&terminal_id, &bytes);
    }
}

pub(super) async fn handle_reaction_toggle(
    ctx: &ClientCtx<'_>,
    _tx: &mut SplitSink<WebSocket, WsMessage>,
    channel: String,
    message_key: String,
    emoji: String,
    user: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let body = crate::transport::rest::ReactionToggle {
        channel,
        message_key,
        emoji,
        user,
    };
    crate::transport::rest::apply_reaction_toggle(ctx.hub, ctx.store, &body).await;
    Ok(())
}
