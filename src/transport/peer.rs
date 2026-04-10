use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use crate::session::Session;
use crate::session::message::Message;

/// Commands from the TUI to the event loop's P2P handler.
#[derive(Debug)]
pub enum P2pCommand {
    CreateSession,
    JoinSession {
        passphrase: String,
        offer_code: String,
    },
    AcceptAnswer {
        answer_code: String,
    },
}

/// Results from async P2P operations back to the TUI.
pub enum P2pResult {
    OfferReady {
        offer: crate::transport::webrtc::PendingOffer,
        code: String,
        passphrase: String,
    },
    AnswerReady {
        channel_pair: crate::transport::webrtc::DataChannelPair,
        code: String,
    },
    Connected {
        channel_pair: crate::transport::webrtc::DataChannelPair,
        peer_name: String,
    },
    Error(String),
}

pub struct RemotePeer {
    pub name: String,
    outbound_tx: mpsc::Sender<String>,
    connected: Arc<std::sync::atomic::AtomicBool>,
}

impl RemotePeer {
    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn send_json(&self, json: &str) -> Result<(), PeerError> {
        self.outbound_tx
            .try_send(json.to_string())
            .map_err(|_| PeerError::Disconnected)
    }
}

#[derive(Debug)]
pub enum PeerError {
    Disconnected,
    Serialization(String),
    Log(String),
}

impl std::fmt::Display for PeerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disconnected => write!(f, "peer disconnected"),
            Self::Serialization(e) => write!(f, "serialization error: {e}"),
            Self::Log(e) => write!(f, "log error: {e}"),
        }
    }
}

impl std::error::Error for PeerError {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PeerMessage {
    #[serde(rename = "hello")]
    Hello { session_id: String, name: String },

    #[serde(rename = "chat")]
    Chat {
        from: String,
        text: String,
        ts: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
        origin: String,
    },

    #[serde(rename = "lesson_sync")]
    LessonSync { lessons_json: String },

    #[serde(rename = "lesson_update")]
    LessonUpdate { lesson_json: String },

    #[serde(rename = "lesson_query")]
    LessonQuery,

    #[serde(rename = "lesson_query_response")]
    LessonQueryResponse { lessons_json: String },
}

/// Spawn relay tasks that bridge a remote peer's transport channel
/// with the local session broadcast.
///
/// Returns the `RemotePeer` handle and a task join handle.
/// The caller provides:
/// - `inbound_rx`: messages arriving from the remote transport
/// - `outbound_tx`: messages to send to the remote transport
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

    // Inbound: remote peer → local session broadcast.
    //
    // `current_name` starts as the placeholder ("remote") and is replaced
    // once the remote side sends its Hello. Lesson storage keys and the
    // disconnect announcement use whatever the most recent name is.
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
                        to: None,
                    };
                    let _ = session_for_inbound.broadcast(&joined);
                }
                PeerMessage::LessonSync { lessons_json } => {
                    if let Ok(remote_index) =
                        serde_json::from_str::<crate::session::lessons::LessonIndex>(&lessons_json)
                    {
                        let _ = session_for_inbound
                            .storage
                            .save_peer_lessons(&current_name, &remote_index);
                    }
                }
                PeerMessage::LessonUpdate { lesson_json } => {
                    if let Ok(lesson) =
                        serde_json::from_str::<crate::session::lessons::AgentLesson>(&lesson_json)
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
                        serde_json::from_str::<crate::session::lessons::LessonIndex>(&lessons_json)
                    {
                        let _ = session_for_inbound
                            .storage
                            .save_peer_lessons(&current_name, &remote_index);
                    }
                }
            }
        }
        // Inbound channel closed — remote peer is gone. Announce so the TUI
        // clears the status dot and channel list.
        let left = Message {
            from: "bpmux".to_string(),
            text: format!("{current_name} left"),
            ts: chrono::Utc::now(),
            message_type: crate::session::MessageType::System,
            to: None,
        };
        let _ = session_for_inbound.broadcast(&left);
        connected_relay.store(false, std::sync::atomic::Ordering::Relaxed);
    });

    // Outbound: local session broadcast → remote peer
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
