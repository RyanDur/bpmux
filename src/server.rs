use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::session;
use crate::transport;

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
    screen_capture: Arc<std::sync::Mutex<String>>,
    inject_tx: tokio::sync::mpsc::Sender<crossterm::event::Event>,
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
        .route(
            "/api/peer-lessons",
            axum::routing::get(transport::rest::get_peer_lessons),
        )
        .route(
            "/api/screen",
            axum::routing::get(transport::rest::get_screen),
        )
        .route(
            "/api/simulate/click",
            axum::routing::post(transport::rest::simulate_click),
        )
        .route(
            "/api/simulate/key",
            axum::routing::post(transport::rest::simulate_key),
        )
        .with_state(session)
        .layer(axum::Extension(screen_capture))
        .layer(axum::Extension(inject_tx));
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
