use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::Response;
use futures::StreamExt;

use super::{WsDeps, run_ws_session};

#[allow(clippy::too_many_arguments)]
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(store): axum::Extension<Arc<store::Store>>,
    axum::Extension(dispatch): axum::Extension<store::Dispatch>,
    axum::Extension(pty_registry): axum::Extension<Arc<room::LivePtyRegistry>>,
    axum::Extension(tracker): axum::Extension<crate::rest::TrackerHandle>,
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
) -> Response {
    let deps = WsDeps {
        hub,
        store,
        dispatch,
        pty_registry,
        tracker,
        orchestrator,
    };
    ws.on_upgrade(move |socket| handle_ws(socket, deps))
}

async fn handle_ws(socket: WebSocket, deps: WsDeps) {
    let (tx, rx) = socket.split();
    if let Err(e) = run_ws_session(tx, rx, deps).await {
        diag::log_diag(&format!("ws: session error: {e}"));
    }
}
