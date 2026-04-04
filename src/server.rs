use std::path::PathBuf;
use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::mcp::BpmuxMcpServer;
use crate::session;
use crate::transport;

pub async fn run_server(
    session_dir: PathBuf,
    peer_name: String,
    wake_tx: broadcast::Sender<session::Message>,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let dir = session_dir.clone();
    let mcp_dir = dir.clone();
    let name = peer_name.clone();
    let tx = wake_tx.clone();

    let service: StreamableHttpService<BpmuxMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || {
                Ok(BpmuxMcpServer::new(
                    mcp_dir.clone(),
                    name.clone(),
                    tx.clone(),
                ))
            },
            Default::default(),
            StreamableHttpServerConfig::default()
                .with_stateful_mode(true)
                .with_cancellation_token(ct.child_token()),
        );

    let ws_state = Arc::new(transport::state::ServerState {
        session_dir: Arc::new(session_dir),
        session_id: dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        wake_tx,
    });

    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::post(transport::rest::post_message),
        )
        .with_state(ws_state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;

    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;

    Ok(())
}
