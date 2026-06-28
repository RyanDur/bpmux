use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{err_body, json_headers};

#[derive(Deserialize)]
pub struct RoadmapQuery {
    pub outcome: Option<u64>,
    pub state: Option<String>,
    pub order: Option<String>,
    pub limit: Option<usize>,
}

pub async fn get_roadmap(
    axum::Extension(orchestrator): axum::Extension<crate::server::OrchestratorHandle>,
    Query(query): Query<RoadmapQuery>,
) -> impl IntoResponse {
    let Some(refresh) = orchestrator.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("roadmap view requires the canonical store"),
        );
    };
    if refresh.all_stories().is_empty() {
        let _ = refresh.refresh_all().await;
    }
    let body = match query.outcome {
        Some(number) => {
            let all = refresh.all_stories();
            let drill = crate::product_management::roadmap::drill::derive_outcome_drill_in(
                number,
                &all,
                crate::workflow::current_workflow(),
            );
            let outcome_name = refresh
                .outcomes()
                .iter()
                .find(|outcome| outcome.id().saved_number() == Some(number))
                .map(|outcome| outcome.content.title.clone());
            serde_json::json!({
                "drill": drill,
                "outcome_number": number,
                "outcome_name": outcome_name,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            })
        }
        None => {
            let state = query
                .state
                .as_deref()
                .and_then(crate::view::roadmap::OutcomeState::from_cli);
            let order = query
                .order
                .as_deref()
                .and_then(crate::view::roadmap_filter::RoadmapOrder::from_cli)
                .unwrap_or(crate::view::roadmap_filter::RoadmapOrder::Priority);
            let filtered = crate::view::roadmap_filter::filter_roadmap(
                crate::view::roadmap::roadmap_cards(refresh),
                state,
                order,
                query.limit,
            );
            serde_json::json!({
                "outcomes": filtered.outcomes,
                "total": filtered.total,
                "shown": filtered.shown,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            })
        }
    };
    (StatusCode::OK, json_headers(), body.to_string())
}
