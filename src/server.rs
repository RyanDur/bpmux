use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::session;
use crate::transport;

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let router = axum::Router::new()
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::get(transport::rest::get_messages).post(transport::rest::post_message),
        )
        .with_state(session);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
