use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::store::CommitmentsAction;
use crate::store::Dispatch;

#[derive(Deserialize)]
pub struct WakeOnEventRequest {
    pub role: String,
    pub event_class: String,
    #[serde(default)]
    pub filter: Option<Map<String, Value>>,
}

pub async fn wake_on_event(
    Extension(dispatch): Extension<Dispatch>,
    Json(payload): Json<WakeOnEventRequest>,
) -> impl IntoResponse {
    dispatch.dispatch(
        CommitmentsAction::Committed {
            role: payload.role,
            event_class: payload.event_class,
            filter: payload.filter,
        }
        .into(),
    );
    (StatusCode::CREATED, "committed".to_string())
}
