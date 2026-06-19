use std::sync::Arc;
use tokio::sync::mpsc;
use webrtc::data_channel::RTCDataChannel;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;

mod peer;

use peer::{
    CHANNEL_BUFFER, DATA_CHANNEL_LABEL, ICE_GATHERING_TIMEOUT_SECS, new_peer_connection,
    register_connection_state_handler, wire_inbound, wire_outbound,
};

#[derive(Debug)]
pub enum WebRtcError {
    Setup(String),
    Sdp(String),
    Connection(String),
    Timeout,
}

impl std::fmt::Display for WebRtcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Setup(e) => write!(f, "WebRTC setup: {e}"),
            Self::Sdp(e) => write!(f, "SDP error: {e}"),
            Self::Connection(e) => write!(f, "connection error: {e}"),
            Self::Timeout => write!(f, "ICE gathering timed out"),
        }
    }
}

impl std::error::Error for WebRtcError {}

pub struct DataChannelPair {
    pub inbound_rx: mpsc::Receiver<String>,
    pub outbound_tx: mpsc::Sender<String>,
    pub peer_connection: Arc<RTCPeerConnection>,
}

pub struct PendingOffer {
    pub offer_sdp: String,
    peer_connection: Arc<RTCPeerConnection>,
    inbound_rx: mpsc::Receiver<String>,
    data_channel: Arc<RTCDataChannel>,
}

impl PendingOffer {
    pub async fn accept_answer(self, answer_sdp: &str) -> Result<DataChannelPair, WebRtcError> {
        let answer: RTCSessionDescription =
            serde_json::from_str(answer_sdp).map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        self.peer_connection
            .set_remote_description(answer)
            .await
            .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        let outbound_tx = wire_outbound(self.data_channel);

        Ok(DataChannelPair {
            inbound_rx: self.inbound_rx,
            outbound_tx,
            peer_connection: self.peer_connection,
        })
    }
}

pub async fn create_offer() -> Result<PendingOffer, WebRtcError> {
    let pc = new_peer_connection().await?;
    let (inbound_tx, inbound_rx) = mpsc::channel(CHANNEL_BUFFER);

    let data_channel = pc
        .create_data_channel(DATA_CHANNEL_LABEL, None)
        .await
        .map_err(|e| WebRtcError::Setup(e.to_string()))?;

    wire_inbound(data_channel.clone(), inbound_tx);
    register_connection_state_handler(&pc);

    let offer = pc
        .create_offer(None)
        .await
        .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    let mut gathering_complete = pc.gathering_complete_promise().await;

    pc.set_local_description(offer)
        .await
        .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    tokio::select! {
        _ = gathering_complete.recv() => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(ICE_GATHERING_TIMEOUT_SECS)) => {
            return Err(WebRtcError::Timeout);
        }
    }

    let local_desc = pc
        .local_description()
        .await
        .ok_or_else(|| WebRtcError::Sdp("no local description after gathering".into()))?;

    let offer_sdp =
        serde_json::to_string(&local_desc).map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    Ok(PendingOffer {
        offer_sdp,
        peer_connection: pc,
        inbound_rx,
        data_channel,
    })
}

pub async fn accept_offer(offer_sdp: &str) -> Result<(String, DataChannelPair), WebRtcError> {
    let pc = new_peer_connection().await?;
    let (inbound_tx, inbound_rx) = mpsc::channel(CHANNEL_BUFFER);

    let inbound_tx_for_dc = inbound_tx.clone();
    let (dc_tx, mut dc_rx) = mpsc::channel::<Arc<RTCDataChannel>>(1);

    pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
        let tx = inbound_tx_for_dc.clone();
        let dc_sender = dc_tx.clone();
        Box::pin(async move {
            wire_inbound(dc.clone(), tx);
            let _ = dc_sender.send(dc).await;
        })
    }));

    register_connection_state_handler(&pc);

    let offer: RTCSessionDescription =
        serde_json::from_str(offer_sdp).map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    pc.set_remote_description(offer)
        .await
        .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    let answer = pc
        .create_answer(None)
        .await
        .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    let mut gathering_complete = pc.gathering_complete_promise().await;

    pc.set_local_description(answer)
        .await
        .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    tokio::select! {
        _ = gathering_complete.recv() => {}
        _ = tokio::time::sleep(std::time::Duration::from_secs(ICE_GATHERING_TIMEOUT_SECS)) => {
            return Err(WebRtcError::Timeout);
        }
    }

    let local_desc = pc
        .local_description()
        .await
        .ok_or_else(|| WebRtcError::Sdp("no local description after gathering".into()))?;

    let answer_sdp =
        serde_json::to_string(&local_desc).map_err(|e| WebRtcError::Sdp(e.to_string()))?;

    let dc = tokio::select! {
        Some(dc) = dc_rx.recv() => dc,
        _ = tokio::time::sleep(std::time::Duration::from_secs(ICE_GATHERING_TIMEOUT_SECS)) => {
            return Err(WebRtcError::Timeout);
        }
    };

    let outbound_tx = wire_outbound(dc);

    Ok((
        answer_sdp,
        DataChannelPair {
            inbound_rx,
            outbound_tx,
            peer_connection: pc,
        },
    ))
}
