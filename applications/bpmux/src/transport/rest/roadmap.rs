use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;

use super::shared::{err_body, json_headers};

#[derive(Deserialize)]
pub struct RoadmapQuery {
    pub outcome: Option<u64>,
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
            let drill =
                crate::board::roadmap_drill::derive_outcome_drill_in(number, &all, workflow);
            let outcome_name = all
                .iter()
                .find(|s| {
                    s.number == number && s.story_type == crate::board::domain::StoryType::Outcome
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
            let bands = crate::board::roadmap_view::derive_roadmap_view(&all, workflow);
            serde_json::json!({
                "bands": bands,
                "fetched_at": chrono::Utc::now().to_rfc3339(),
            })
        }
    };
    (StatusCode::OK, json_headers(), body.to_string())
}
