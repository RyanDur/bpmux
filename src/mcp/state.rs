use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::broadcast;

use crate::session;

#[derive(Clone)]
pub struct ServerState {
    pub session_dir: Arc<PathBuf>,
    pub session_id: String,
    pub wake_tx: broadcast::Sender<session::Message>,
}
