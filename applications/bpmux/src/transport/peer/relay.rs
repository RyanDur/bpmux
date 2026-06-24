use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use super::{PeerMessage, RemotePeer};
use crate::session::Session;
use crate::session::message::Message;

pub fn spawn_relay(
    session: Arc<Session>,
    peer_name: String,
    local_session_id: String,
    mut inbound_rx: mpsc::Receiver<String>,
    outbound_tx: mpsc::Sender<String>,
) -> RemotePeer {
    let connected = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let connected_relay = connected.clone();
    let session_for_inbound = session.clone();
    let local_id = local_session_id.clone();
    let reply_tx = outbound_tx.clone();
    let initial_peer_name = peer_name.clone();

    tokio::spawn(async move {
        let mut current_name = initial_peer_name;
        while let Some(json) = inbound_rx.recv().await {
            let msg: PeerMessage = match serde_json::from_str(&json) {
                Ok(m) => m,
                Err(_) => continue,
            };

            match msg {
                PeerMessage::Chat {
                    from,
                    text,
                    ts,
                    to,
                    addressee,
                    message_type,
                    origin,
                } => {
                    if origin == local_id {
                        continue;
                    }
                    let message = Message {
                        from,
                        text,
                        ts: ts.parse().unwrap_or_else(|_| chrono::Utc::now()),
                        message_type: message_type.as_deref().into(),
                        to,
                        addressee,
                        ..Message::default()
                    };
                    let _ = session_for_inbound.broadcast(&message);
                }
                PeerMessage::Hello { name, .. } => {
                    current_name = name.clone();
                    let joined = Message {
                        from: "bpmux".to_string(),
                        text: format!("{name} joined"),
                        ts: chrono::Utc::now(),
                        message_type: crate::session::MessageType::System,
                        ..Message::default()
                    };
                    let _ = session_for_inbound.broadcast(&joined);
                }
                PeerMessage::LessonSync { lessons_json } => {
                    if let Ok(remote_index) =
                        serde_json::from_str::<crate::memory::lessons::LessonIndex>(&lessons_json)
                    {
                        let _ = session_for_inbound
                            .storage
                            .save_peer_lessons(&current_name, &remote_index);
                    }
                }
                PeerMessage::LessonUpdate { lesson_json } => {
                    if let Ok(lesson) =
                        serde_json::from_str::<crate::memory::lessons::AgentLesson>(&lesson_json)
                    {
                        let mut peer_index =
                            session_for_inbound.storage.load_peer_lessons(&current_name);
                        let existing = peer_index.lessons.iter_mut().find(|l| {
                            l.topic == lesson.topic && l.author_role == lesson.author_role
                        });
                        if let Some(existing) = existing {
                            *existing = lesson;
                        } else {
                            peer_index.lessons.push(lesson);
                        }
                        let _ = session_for_inbound
                            .storage
                            .save_peer_lessons(&current_name, &peer_index);
                    }
                }
                PeerMessage::LessonQuery => {
                    let local_index = session_for_inbound.storage.load_lessons();
                    if let Ok(lessons_json) = serde_json::to_string(&local_index) {
                        let response = PeerMessage::LessonQueryResponse { lessons_json };
                        if let Ok(json) = serde_json::to_string(&response) {
                            let _ = reply_tx.send(json).await;
                        }
                    }
                }
                PeerMessage::LessonQueryResponse { lessons_json } => {
                    if let Ok(remote_index) =
                        serde_json::from_str::<crate::memory::lessons::LessonIndex>(&lessons_json)
                    {
                        let _ = session_for_inbound
                            .storage
                            .save_peer_lessons(&current_name, &remote_index);
                    }
                }
            }
        }
        let left = Message {
            from: "bpmux".to_string(),
            text: format!("{current_name} left"),
            ts: chrono::Utc::now(),
            message_type: crate::session::MessageType::System,
            ..Message::default()
        };
        let _ = session_for_inbound.broadcast(&left);
        connected_relay.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    let mut session_rx = session.subscribe();
    let outbound_for_relay = outbound_tx.clone();
    let connected_outbound = connected.clone();

    tokio::spawn(async move {
        loop {
            match session_rx.recv().await {
                Ok(msg) => {
                    let peer_msg = PeerMessage::Chat {
                        from: msg.from,
                        text: msg.text,
                        ts: msg.ts.to_rfc3339(),
                        to: msg.to,
                        addressee: msg.addressee,
                        message_type: msg.message_type.to_wire(),
                        origin: local_session_id.clone(),
                    };
                    let json = match serde_json::to_string(&peer_msg) {
                        Ok(j) => j,
                        Err(_) => continue,
                    };
                    if outbound_for_relay.send(json).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
        connected_outbound.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    RemotePeer {
        name: peer_name,
        outbound_tx,
        connected,
    }
}
