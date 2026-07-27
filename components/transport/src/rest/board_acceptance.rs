use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use axum::http::HeaderMap;

use super::shared::{err_body, json_headers, owner_walk_or_forbidden};

#[derive(Deserialize)]
pub struct AcceptPayload {
    pub would_file_bug: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub bypass_reason: Option<String>,
}

pub async fn post_accept_story(
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
    headers: HeaderMap,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<AcceptPayload>,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers, "board acceptance") {
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
    let bypass = match bridge::tools_stories::verify_bug_question_or_bypass(
        &args,
        bridge::tools_stories::BugGate::Accept,
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
    match bridge::tools_stories::accept_via_canonical_store(
        &refresh,
        &select,
        number,
        "ui-electron",
        None,
        bypass,
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            json_headers(),
            serde_json::json!({ "ok": true }).to_string(),
        ),
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
    axum::Extension(orchestrator): axum::Extension<sync::OrchestratorHandle>,
    axum::Extension(select): axum::Extension<store::Select>,
    headers: HeaderMap,
    axum::extract::Path(number): axum::extract::Path<u64>,
    Json(payload): Json<RejectPayload>,
) -> impl IntoResponse {
    if let Err(refusal) = owner_walk_or_forbidden(&select, &headers, "board acceptance") {
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
    match bridge::tools_stories::reject_via_canonical_store(
        &refresh,
        &select,
        number,
        "ui-electron",
        None,
        note,
    )
    .await
    {
        Ok(()) => (
            StatusCode::OK,
            json_headers(),
            serde_json::json!({ "ok": true }).to_string(),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            json_headers(),
            err_body(&format!("reject failed: {e}")),
        ),
    }
}
