use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use crate::session;
use crate::transport;
use crate::transport::auth::AuthState;

pub async fn run_server(
    session: Arc<session::Session>,
    port: u16,
    ct: CancellationToken,
    screen_capture: Arc<std::sync::Mutex<String>>,
    inject_tx: tokio::sync::mpsc::Sender<crate::terminal::event::TerminalInjectEvent>,
) -> anyhow::Result<()> {
    // Ensure the per-run auth token exists before binding the port.
    // Failing here is loud and blocks startup — simulate endpoints
    // can't be gated without a secret.
    let token_path = session::auth_token::default_auth_token_path()
        .ok_or_else(|| anyhow::anyhow!("cannot locate ~/.bpmux auth token path"))?;
    let token = session::auth_token::ensure(&token_path).map_err(|e| {
        anyhow::anyhow!(
            "Unable to write auth token to {}: {e}",
            token_path.display()
        )
    })?;
    let auth_state = AuthState::new(token);

    // Simulate routes live on a sub-router with the auth middleware
    // applied. Other endpoints (messages, search, screen, lessons)
    // stay open — they're read-mostly and were never identified as
    // inject vectors.
    let simulate_routes = axum::Router::new()
        .route(
            "/api/simulate/click",
            axum::routing::post(transport::rest::simulate_click),
        )
        .route(
            "/api/simulate/key",
            axum::routing::post(transport::rest::simulate_key),
        )
        .layer(axum::middleware::from_fn_with_state(
            auth_state,
            transport::auth::gate,
        ));

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
            "/api/classifier-misfire",
            axum::routing::post(transport::rest::post_classifier_misfire),
        )
        .merge(simulate_routes)
        .with_state(session)
        .layer(axum::Extension(screen_capture))
        .layer(axum::Extension(inject_tx));
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    axum::serve(listener, router)
        .with_graceful_shutdown(async move { ct.cancelled_owned().await })
        .await?;
    Ok(())
}
