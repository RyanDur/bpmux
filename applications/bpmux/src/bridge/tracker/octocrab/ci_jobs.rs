use super::OctocrabStoryTracker;

impl OctocrabStoryTracker {
    pub async fn fetch_job_status_for_run(
        &self,
        run_id: u64,
    ) -> Result<Vec<crate::ci::CiJobRecord>, crate::ci::CiJobStatusError> {
        #[derive(serde::Deserialize)]
        struct Job {
            name: String,
            conclusion: Option<String>,
            status: Option<String>,
            started_at: Option<String>,
            completed_at: Option<String>,
            html_url: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct JobsResponse {
            jobs: Vec<Job>,
        }
        let jobs_path = format!(
            "/repos/{}/{}/actions/runs/{run_id}/jobs?per_page=100",
            self.owner, self.repo
        );
        let response: JobsResponse = self
            .client
            .get(jobs_path, None::<&()>)
            .await
            .map_err(|e| classify_jobs_error(&e.to_string(), run_id))?;
        let records: Vec<crate::ci::CiJobRecord> = response
            .jobs
            .into_iter()
            .map(|j| crate::ci::CiJobRecord {
                name: j.name,
                conclusion: j
                    .conclusion
                    .unwrap_or_else(|| j.status.unwrap_or_else(|| "unknown".to_string())),
                duration_ms: crate::ci::compute_duration_ms(
                    j.started_at.as_deref(),
                    j.completed_at.as_deref(),
                ),
                started_at: j.started_at,
                completed_at: j.completed_at,
                html_url: j.html_url.unwrap_or_default(),
            })
            .collect();
        Ok(records)
    }

    pub async fn rerun_failed_jobs(&self, run_id: u64) -> Result<(), String> {
        let path = format!(
            "/repos/{}/{}/actions/runs/{run_id}/rerun-failed-jobs",
            self.owner, self.repo
        );
        let response = self
            .client
            ._post(path, None::<&serde_json::Value>)
            .await
            .map_err(|e| format!("rerun_failed_jobs run_id={run_id}: {e}"))?;
        let status = response.status();
        if status.is_success() {
            Ok(())
        } else {
            Err(format!(
                "rerun_failed_jobs run_id={run_id}: github status {status}"
            ))
        }
    }
}

fn classify_jobs_error(msg: &str, run_id: u64) -> crate::ci::CiJobStatusError {
    if msg.contains("404") {
        crate::ci::CiJobStatusError::not_found(run_id)
    } else if msg.contains("401") || msg.contains("403") {
        crate::ci::CiJobStatusError::auth_failed()
    } else if msg.contains("429") {
        crate::ci::CiJobStatusError::rate_limit(60)
    } else if msg.contains("spending limit") || msg.contains("quota") {
        crate::ci::CiJobStatusError::quota_exhausted()
    } else {
        crate::ci::CiJobStatusError::network_error(msg.to_string())
    }
}
