use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use axum::http::HeaderMap;

use super::shared::{
    OWNER_NONCE_HEADER, broadcast_board_invalidate, broadcast_stories, err_body, json_headers,
};
use crate::hub;

type JsonResponse = (
    StatusCode,
    [(axum::http::HeaderName, &'static str); 1],
    String,
);

fn owner_walk_or_forbidden(
    select: &crate::store::Select,
    headers: &HeaderMap,
) -> Result<(), JsonResponse> {
    let claimed = select.select(|s| crate::owner_session::claimed_nonce(&s.owner_session));
    let presented = headers
        .get(OWNER_NONCE_HEADER)
        .and_then(|value| value.to_str().ok());
    if crate::owner_session::owner_walk_authorized(claimed.as_deref(), presented) {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            json_headers(),
            err_body(
                "board acceptance requires the owner session — attach the current launch nonce",
            ),
        ))
    }
}

#[derive(Deserialize)]
pub struct AcceptPayload {
    pub would_file_bug: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub bypass_reason: Option<String>,
}

pub async fn post_accept_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    headers: HeaderMap,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<AcceptPayload>,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers) {
        return refusal;
    }
    let mut args = serde_json::Map::new();
    args.insert(
        "would_file_bug".to_string(),
        serde_json::Value::Bool(payload.would_file_bug),
    );
    args.insert("force".to_string(), serde_json::Value::Bool(payload.force));
    if let Some(r) = payload.bypass_reason.as_ref() {
        args.insert(
            "bypass_reason".to_string(),
            serde_json::Value::String(r.clone()),
        );
    }
    let bypass = match crate::bridge::tools_stories::verify_bug_question_or_bypass(
        &args,
        crate::bridge::tools_stories::BugGate::Accept,
    ) {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, json_headers(), err_body(&e));
        }
    };
    let Some(refresh) = orchestrator.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match crate::bridge::tools_stories::accept_via_canonical_store(
        &refresh,
        number,
        "ui-electron",
        None,
        bypass,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&hub);
            broadcast_stories(&hub, &refresh);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("accept failed: {e}")),
        ),
    }
}

#[derive(Deserialize)]
pub struct RejectPayload {
    pub note: String,
}

pub async fn post_reject_story(
    State(hub): State<Arc<hub::Hub>>,
    axum::Extension(orchestrator): axum::Extension<crate::sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<crate::store::Select>,
    headers: HeaderMap,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<RejectPayload>,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers) {
        return refusal;
    }
    let note = payload.note.trim().to_string();
    if note.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            err_body("note must not be empty"),
        );
    }
    let Some(refresh) = orchestrator.as_ref().cloned() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("orchestrator not configured"),
        );
    };
    match crate::bridge::tools_stories::reject_via_canonical_store(
        &refresh,
        number,
        "ui-electron",
        None,
        note,
    )
    .await
    {
        Ok(()) => {
            broadcast_board_invalidate(&hub);
            broadcast_stories(&hub, &refresh);
            (
                StatusCode::OK,
                json_headers(),
                serde_json::json!({ "ok": true }).to_string(),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("reject failed: {e}")),
        ),
    }
}
