use std::sync::Arc;
use tokio::sync::mpsc;
use webrtc::api::APIBuilder;
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::data_channel::RTCDataChannel;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;

use super::WebRtcError;

pub(super) const DATA_CHANNEL_LABEL: &str = "bpmux";
pub(super) const CHANNEL_BUFFER: usize = 256;
pub(super) const ICE_GATHERING_TIMEOUT_SECS: u64 = 10;

pub(super) async fn new_peer_connection() -> Result<Arc<RTCPeerConnection>, WebRtcError> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| WebRtcError::Setup(e.to_string()))?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)
        .map_err(|e| WebRtcError::Setup(e.to_string()))?;

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();

    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };

    let pc = api
        .new_peer_connection(config)
        .await
        .map_err(|e| WebRtcError::Setup(e.to_string()))?;

    Ok(Arc::new(pc))
}

pub(super) fn wire_inbound(dc: Arc<RTCDataChannel>, tx: mpsc::Sender<String>) {
    dc.on_message(Box::new(move |msg: DataChannelMessage| {
        let tx = tx.clone();
        let text = String::from_utf8(msg.data.to_vec()).unwrap_or_default();
        Box::pin(async move {
            let _ = tx.send(text).await;
        })
    }));
}

pub(super) fn wire_outbound(dc: Arc<RTCDataChannel>) -> mpsc::Sender<String> {
    let (tx, mut rx) = mpsc::channel::<String>(CHANNEL_BUFFER);

    tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if dc.send_text(text).await.is_err() {
                break;
            }
        }
    });

    tx
}

pub(super) fn register_connection_state_handler(pc: &Arc<RTCPeerConnection>) {
    pc.on_peer_connection_state_change(Box::new(|state: RTCPeerConnectionState| {
        Box::pin(async move {
            match state {
                RTCPeerConnectionState::Failed | RTCPeerConnectionState::Disconnected => {}
                _ => {}
            }
        })
    }));
}
