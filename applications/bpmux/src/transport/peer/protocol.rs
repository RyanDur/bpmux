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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum PeerMessage {
    #[serde(rename = "hello")]
    Hello { session_id: String, name: String },

    #[serde(rename = "chat")]
    Chat {
        from: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sender_id: Option<String>,
        text: String,
        ts: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee_ids: Option<Vec<String>>,
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
