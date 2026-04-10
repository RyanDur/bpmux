//! P2P connection handling — command dispatch and result processing.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::session;
use crate::terminal::app::App;
use crate::transport::crypto;
use crate::transport::peer::{self, P2pCommand, P2pResult};
use crate::transport::webrtc;

pub(super) fn handle_p2p_command(
    cmd: P2pCommand,
    tx: &mpsc::Sender<P2pResult>,
    active_passphrase: &mut Option<String>,
    pending_offer: &mut Option<webrtc::PendingOffer>,
) {
    let tx = tx.clone();
    match cmd {
        P2pCommand::CreateSession => {
            tokio::spawn(async move {
                let passphrase = match crypto::generate_passphrase() {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                        return;
                    }
                };

                match webrtc::create_offer().await {
                    Ok(offer) => {
                        let sdp = offer.offer_sdp.clone();
                        match crypto::encrypt(&sdp, &passphrase) {
                            Ok(code) => {
                                let _ = tx
                                    .send(P2pResult::OfferReady {
                                        offer,
                                        code,
                                        passphrase,
                                    })
                                    .await;
                            }
                            Err(e) => {
                                let _ = tx.send(P2pResult::Error(e.to_string())).await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                    }
                }
            });
        }
        P2pCommand::JoinSession {
            passphrase,
            offer_code,
        } => {
            tokio::spawn(async move {
                let offer_sdp = match crypto::decrypt(&offer_code, &passphrase) {
                    Ok(sdp) => sdp,
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                        return;
                    }
                };

                match webrtc::accept_offer(&offer_sdp).await {
                    Ok((answer_sdp, channel_pair)) => {
                        match crypto::encrypt(&answer_sdp, &passphrase) {
                            Ok(code) => {
                                let _ =
                                    tx.send(P2pResult::AnswerReady { channel_pair, code }).await;
                            }
                            Err(e) => {
                                let _ = tx.send(P2pResult::Error(e.to_string())).await;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                    }
                }
            });
        }
        P2pCommand::AcceptAnswer { answer_code } => {
            let Some(passphrase) = active_passphrase.clone() else {
                let _ = tx.try_send(P2pResult::Error(
                    "No active session. Run /session first.".to_string(),
                ));
                return;
            };
            let Some(offer) = pending_offer.take() else {
                let _ = tx.try_send(P2pResult::Error(
                    "No pending offer. Run /session first.".to_string(),
                ));
                return;
            };
            tokio::spawn(async move {
                let answer_sdp = match crypto::decrypt(&answer_code, &passphrase) {
                    Ok(sdp) => sdp,
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                        return;
                    }
                };

                match offer.accept_answer(&answer_sdp).await {
                    Ok(channel_pair) => {
                        let _ = tx
                            .send(P2pResult::Connected {
                                channel_pair,
                                peer_name: "remote".to_string(),
                            })
                            .await;
                    }
                    Err(e) => {
                        let _ = tx.send(P2pResult::Error(e.to_string())).await;
                    }
                }
            });
        }
    }
}

async fn send_hello(local_name: &str, local_session_id: &str, outbound: &mpsc::Sender<String>) {
    let msg = peer::PeerMessage::Hello {
        session_id: local_session_id.to_string(),
        name: local_name.to_string(),
    };
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = outbound.send(json).await;
    }
}

async fn send_lesson_sync(session: &Arc<session::Session>, outbound: &mpsc::Sender<String>) {
    let index = session.storage.load_lessons();
    if let Ok(lessons_json) = serde_json::to_string(&index) {
        let msg = peer::PeerMessage::LessonSync { lessons_json };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = outbound.send(json).await;
        }
    }
}

pub(super) fn handle_p2p_result(
    result: P2pResult,
    app: &mut App,
    session_arc: &Arc<session::Session>,
    active_passphrase: &mut Option<String>,
    pending_offer: &mut Option<webrtc::PendingOffer>,
) {
    match result {
        P2pResult::OfferReady {
            offer,
            code,
            passphrase,
        } => {
            *active_passphrase = Some(passphrase.clone());
            *pending_offer = Some(offer);
            app.system_message(format!(
                "Session ready. Share these with the other machine:\n\n  Passphrase: {passphrase}\n  Offer code: {code}\n\nWaiting for /accept <answer-code>"
            ));
        }
        P2pResult::AnswerReady { channel_pair, code } => {
            spawn_relay_with_sync(
                session_arc.clone(),
                app.session_id.clone(),
                app.peer_name.clone(),
                "remote".to_string(),
                channel_pair,
            );
            app.system_message(format!(
                "Connected to session. Share this answer code with the offerer:\n\n  Answer code: {code}\n\nPeer connected."
            ));
        }
        P2pResult::Connected {
            channel_pair,
            peer_name,
        } => {
            spawn_relay_with_sync(
                session_arc.clone(),
                app.session_id.clone(),
                app.peer_name.clone(),
                peer_name.clone(),
                channel_pair,
            );
            app.system_message(format!("Connected to peer '{peer_name}'."));
        }
        P2pResult::Error(e) => {
            app.system_message(format!("P2P error: {e}"));
        }
    }
}

pub(super) fn spawn_relay_with_sync(
    session: Arc<session::Session>,
    session_id: String,
    local_name: String,
    peer_name: String,
    pair: webrtc::DataChannelPair,
) {
    tokio::spawn(async move {
        send_hello(&local_name, &session_id, &pair.outbound_tx).await;
        send_lesson_sync(&session, &pair.outbound_tx).await;
        let _peer = peer::spawn_relay(
            session,
            peer_name,
            session_id,
            pair.inbound_rx,
            pair.outbound_tx,
        );
        std::future::pending::<()>().await;
    });
}
