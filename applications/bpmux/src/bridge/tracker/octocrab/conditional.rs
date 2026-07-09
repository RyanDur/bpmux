use crate::bridge::tracker::WorkflowRunEntry;

use super::{ConditionalRunEntry, OctocrabStoryTracker};

impl OctocrabStoryTracker {
    pub async fn list_latest_run_on_main_conditional(
        &self,
        prior_etag: Option<&str>,
    ) -> Result<ConditionalRunEntry, String> {
        use http_body_util::BodyExt;
        let path = format!(
            "/repos/{}/{}/actions/runs?branch=main&per_page=20",
            self.owner, self.repo
        );
        let mut headers = http::HeaderMap::new();
        if let Some(etag) = prior_etag
            && let Ok(value) = http::HeaderValue::from_str(etag)
        {
            headers.insert("if-none-match", value);
        }
        let response = self
            .client
            ._get_with_headers(path, Some(headers))
            .await
            .map_err(Self::classify)?;
        crate::rate_limit::capture_headers(
            response.headers(),
            &self.rate_limit_observer,
            std::time::SystemTime::now(),
        );
        let status = response.status();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string);
        if status.as_u16() == 304 {
            return Ok(ConditionalRunEntry::NotModified);
        }
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("body collect: {e}"))?
            .to_bytes();
        if !status.is_success() {
            return Err(format!(
                "github error {status}: {}",
                String::from_utf8_lossy(&body)
            ));
        }
        #[derive(serde::Deserialize)]
        struct RunsResponse {
            workflow_runs: Vec<WorkflowRunEntry>,
        }
        let parsed: RunsResponse =
            serde_json::from_slice(&body).map_err(|e| format!("json parse: {e}"))?;
        let mut entries = parsed.workflow_runs;
        if entries.is_empty() {
            return Ok(ConditionalRunEntry::Empty);
        }
        let head_sha = entries[0].head_sha.clone();
        for entry in entries
            .iter_mut()
            .filter(|e| e.head_sha == head_sha)
            .filter(|e| e.conclusion.as_deref() == Some("failure"))
        {
            if let Some(run_id) = entry.database_id {
                entry.annotation_message = self.fetch_first_failure_annotation(run_id).await;
            }
        }
        Ok(ConditionalRunEntry::Modified { entries, etag })
    }
}
