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
    axum::Extension(test_refresh): axum::Extension<crate::server::TestRefreshHandle>,
    Query(query): Query<RoadmapQuery>,
) -> impl IntoResponse {
    let Some(refresh) = test_refresh.as_ref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            json_headers(),
            err_body("roadmap view requires the canonical store"),
        );
    };
    if refresh.all_stories().is_empty() {
        let _ = refresh.refresh_all().await;
    }
    let all = refresh.all_stories();
    let workflow = crate::workflow::current_workflow();
    let body = match query.outcome {
        Some(number) => {
            let drill = crate::product_management::roadmap::drill::derive_outcome_drill_in(
                number, &all, workflow,
            );
            let outcome_name = all
                .iter()
                .find(|s| {
                    s.number == number
                        && s.story_type
                            == crate::product_management::board::domain::StoryType::Outcome
                })
                .map(|s| s.title.clone());
            serde_json::json!({
                "drill": drill,
                "outcome_number": number,
                "outcome_name": outcome_name,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            })
        }
        None => {
            let bands =
                crate::product_management::roadmap::view::derive_roadmap_view(&all, workflow);
            let filter = crate::product_management::roadmap::filter::RoadmapQuery {
                state: query
                    .state
                    .as_deref()
                    .and_then(crate::product_management::roadmap::view::OutcomeState::from_cli),
                order: query
                    .order
                    .as_deref()
                    .and_then(crate::product_management::roadmap::filter::RoadmapOrder::from_cli)
                    .unwrap_or(crate::product_management::roadmap::filter::RoadmapOrder::Priority),
                limit: query.limit,
            };
            let filtered =
                crate::product_management::roadmap::filter::apply_roadmap_query(bands, &filter);
            serde_json::json!({
                "bands": filtered.bands,
                "total": filtered.total,
                "shown": filtered.shown,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            })
        }
    };
    (StatusCode::OK, json_headers(), body.to_string())
}
