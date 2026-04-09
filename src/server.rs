use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::session;
use crate::transport;

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    lan: bool,
    ct: CancellationToken,
) -> anyhow::Result<()> {
    let router = axum::Router::new()
        .route("/ws", axum::routing::any(transport::ws::ws_handler))
        .route(
            "/api/messages",
            axum::routing::get(transport::rest::get_messages).post(transport::rest::post_message),
        )
        .route(
            "/api/search",
            axum::routing::get(transport::rest::search_messages),
        )
        .route(
            "/api/lessons",
            axum::routing::get(transport::rest::get_lessons).post(transport::rest::post_lesson),
        )
        .with_state(session);
    let bind_addr = if lan { "0.0.0.0" } else { "127.0.0.1" };
    let listener = tokio::net::TcpListener::bind(format!("{bind_addr}:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
