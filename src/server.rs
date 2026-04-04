use std::sync::Arc;

use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;

use crate::mcp::BpmuxMcpServer;
use crate::session;
use crate::transport;

pub async fn run_server(
    session: Arc<session::Session>,
    peer_name: String,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let mcp_session = Arc::clone(&session);
    let name = peer_name.clone();
    let service: StreamableHttpService<BpmuxMcpServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || Ok(BpmuxMcpServer::new(Arc::clone(&mcp_session), name.clone())),
            Default::default(),
            StreamableHttpServerConfig::default()
                .with_stateful_mode(true)
                .with_cancellation_token(ct.child_token()),
        );
    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::post(transport::rest::post_message),
        )
        .with_state(session);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
