use axum::Json;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::store::CommitmentsAction;
use crate::store::{Dispatch, Select};
use user_routing::short_name;

#[derive(Deserialize)]
pub struct WakeOnEventRequest {
    pub role: String,
    #[serde(default)]
    pub user_id: Option<String>,
    pub event_class: String,
    #[serde(default)]
    pub filter: Option<Map<String, Value>>,
}

pub async fn wake_on_event(
    Extension(dispatch): Extension<Dispatch>,
    Extension(select): Extension<Select>,
    Json(payload): Json<WakeOnEventRequest>,
) -> impl IntoResponse {
    let user_ids = match payload.user_id {
        Some(user_id) => vec![user_id],
        None => {
            select.select(|s| crate::store::routing::resolve(&s.routing, short_name(&payload.role)))
        }
    };
    for user_id in user_ids {
        dispatch.dispatch(
            CommitmentsAction::Committed {
                user_id,
                event_class: payload.event_class.clone(),
                filter: payload.filter.clone(),
            }
            .into(),
        );
    }
    (StatusCode::CREATED, "committed".to_string())
}
