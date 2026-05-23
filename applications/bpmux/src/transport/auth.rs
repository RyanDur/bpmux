use axum::{
    extract::State,
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;
use std::sync::Arc;

use crate::session::auth_token;

#[derive(Clone)]
pub struct AuthState {
    pub token: Arc<String>,
}

impl AuthState {
    pub fn new(token: String) -> Self {
        Self {
            token: Arc::new(token),
        }
    }
}

pub async fn gate(
    State(state): State<AuthState>,
    headers: HeaderMap,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    if !is_authorized(&headers, &state.token) {
        return unauthorized();
    }
    next.run(req).await
}

pub fn is_authorized(headers: &HeaderMap, expected_token: &str) -> bool {
    let bearer_ok = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_bearer)
        .is_some_and(|got| auth_token::constant_time_eq(got, expected_token));
    let host_ok = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .is_some_and(is_loopback_host);
    bearer_ok && host_ok
}

fn parse_bearer(value: &str) -> Option<&str> {
    let rest = value.strip_prefix("Bearer ")?;
    let rest = rest.trim();
    if rest.is_empty() { None } else { Some(rest) }
}

pub fn is_loopback_host(host: &str) -> bool {
    let host_only = match host.rsplit_once(':') {
        Some(_) if host.starts_with('[') => {
            if let Some(idx) = host.rfind(']') {
                &host[..=idx]
            } else {
                host
            }
        }
        Some((h, _)) => h,
        None => host,
    };
    matches!(host_only, "127.0.0.1" | "localhost" | "[::1]")
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::CONTENT_TYPE, "application/json")],
        json!({"error": "unauthorized"}).to_string(),
    )
        .into_response()
}
