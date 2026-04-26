use std::sync::Arc;

use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use crate::board::RefreshDriver;
use crate::bridge;
use crate::server::TestRefreshHandle;

#[derive(Debug, Deserialize)]
pub struct CreateStoryReq {
    pub title: String,
    pub user_job: String,
    pub observation_falsifier: String,
    pub body: Option<String>,
    pub parent_epic: Option<u64>,
    pub agent: Option<String>,
}

pub async fn post_test_create_story(
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CreateStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert("title".to_string(), serde_json::Value::String(req.title));
    args.insert(
        "user_job".to_string(),
        serde_json::Value::String(req.user_job),
    );
    args.insert(
        "observation_falsifier".to_string(),
        serde_json::Value::String(req.observation_falsifier),
    );
    if let Some(body) = req.body {
        args.insert("body".to_string(), serde_json::Value::String(body));
    }
    if let Some(epic) = req.parent_epic {
        args.insert(
            "parent_epic".to_string(),
            serde_json::Value::Number(epic.into()),
        );
    }
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_create_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => (
            StatusCode::OK,
            Json(serde_json::json!({ "message": message })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CommentOnStoryReq {
    pub number: u64,
    pub body: String,
    pub agent: Option<String>,
}

pub async fn post_test_comment_on_story(
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CommentOnStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert("body".to_string(), serde_json::Value::String(req.body));
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_comment_on_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => (
            StatusCode::OK,
            Json(serde_json::json!({ "message": message })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct CloseStoryReq {
    pub number: u64,
    pub reason: String,
    pub agent: Option<String>,
}

pub async fn post_test_close_story(
    axum::Extension(refresh): axum::Extension<TestRefreshHandle>,
    Json(req): Json<CloseStoryReq>,
) -> impl IntoResponse {
    let Some(refresh) = refresh else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({
                "error": "BPMUX_TEST_MODE not enabled — test routes inactive"
            })),
        );
    };
    let mut args = serde_json::Map::new();
    args.insert(
        "number".to_string(),
        serde_json::Value::Number(req.number.into()),
    );
    args.insert("reason".to_string(), serde_json::Value::String(req.reason));
    let agent = req.agent.unwrap_or_else(|| "pm:test".to_string());
    let refresh_ref: &Arc<dyn RefreshDriver> = &refresh;
    match bridge::dispatch_test_close_story(args, &agent, Some(refresh_ref)).await {
        Ok(message) => (
            StatusCode::OK,
            Json(serde_json::json!({ "message": message })),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e })),
        ),
    }
}
