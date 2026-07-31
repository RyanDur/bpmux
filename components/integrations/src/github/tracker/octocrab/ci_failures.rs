use super::OctocrabStoryTracker;

impl OctocrabStoryTracker {
    pub(super) async fn fetch_first_failure_annotation(&self, run_id: u64) -> Option<String> {
        #[derive(serde::Deserialize)]
        struct Job {
            check_run_url: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct JobsResponse {
            jobs: Vec<Job>,
        }
        let jobs_path = format!(
            "/repos/{}/{}/actions/runs/{run_id}/jobs",
            self.owner, self.repo
        );
        let jobs: JobsResponse = match self.client.get(jobs_path, None::<&()>).await {
            Ok(r) => r,
            Err(e) => {
                diag::log_diag(&format!("tracker: fetch jobs for run {run_id} failed: {e}"));
                return None;
            }
        };
        let check_run_url = jobs.jobs.into_iter().find_map(|j| j.check_run_url)?;
        #[derive(serde::Deserialize)]
        struct CheckRun {
            output: Option<CheckRunOutput>,
        }
        #[derive(serde::Deserialize)]
        struct CheckRunOutput {
            annotations_url: Option<String>,
        }
        let check_run: CheckRun = match self.client.get(check_run_url.as_str(), None::<&()>).await {
            Ok(r) => r,
            Err(e) => {
                diag::log_diag(&format!(
                    "tracker: fetch check-run failed for run {run_id}: {e}"
                ));
                return None;
            }
        };
        let annotations_url = check_run.output.and_then(|o| o.annotations_url)?;
        #[derive(serde::Deserialize)]
        struct Annotation {
            message: Option<String>,
        }
        let annotations: Vec<Annotation> =
            match self.client.get(annotations_url.as_str(), None::<&()>).await {
                Ok(r) => r,
                Err(e) => {
                    diag::log_diag(&format!(
                        "tracker: fetch annotations failed for run {run_id}: {e}"
                    ));
                    return None;
                }
            };
        annotations.into_iter().find_map(|a| a.message)
    }

    pub async fn fetch_failed_tests_for_run(
        &self,
        run_id: u64,
    ) -> Result<ci::CiRunFailures, ci::CiLogError> {
        #[derive(serde::Deserialize)]
        struct Run {
            status: Option<String>,
            conclusion: Option<String>,
        }
        let run_path = format!("/repos/{}/{}/actions/runs/{run_id}", self.owner, self.repo);
        let run: Run = self.client.get(run_path, None::<&()>).await.map_err(|e| {
            let msg = format!("{e}");
            if msg.contains("404") {
                ci::CiLogError::RunNotFound { run_id }
            } else {
                ci::CiLogError::UpstreamError { detail: msg }
            }
        })?;
        let conclusion = run
            .conclusion
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if run.status.as_deref() != Some("completed") {
            return Err(ci::CiLogError::RunInProgress {
                run_id,
                conclusion: run
                    .status
                    .or(run.conclusion)
                    .unwrap_or_else(|| "in_progress".to_string()),
            });
        }
        #[derive(serde::Deserialize)]
        struct Job {
            id: u64,
            name: String,
            conclusion: Option<String>,
        }
        #[derive(serde::Deserialize)]
        struct JobsResponse {
            jobs: Vec<Job>,
        }
        let jobs_path = format!(
            "/repos/{}/{}/actions/runs/{run_id}/jobs?per_page=100",
            self.owner, self.repo
        );
        let jobs: JobsResponse = self.client.get(jobs_path, None::<&()>).await.map_err(|e| {
            ci::CiLogError::UpstreamError {
                detail: format!("fetch jobs for run {run_id}: {e}"),
            }
        })?;
        let mut all_failures: Vec<ci::CiTestFailure> = Vec::new();
        for job in jobs.jobs {
            if job.conclusion.as_deref() != Some("failure") {
                continue;
            }
            let Some(log_text) = self.fetch_job_log(job.id).await else {
                continue;
            };
            all_failures.extend(job_failures(run_id, &job.name, &log_text));
        }
        let final_failures = if all_failures.is_empty() && conclusion == "failure" {
            vec![workflow_failure_fallback(run_id)]
        } else {
            all_failures
        };
        Ok(ci::CiRunFailures {
            run_id,
            conclusion,
            failures: final_failures,
        })
    }

    async fn fetch_job_log(&self, job_id: u64) -> Option<String> {
        use http_body_util::BodyExt;
        let log_path = format!(
            "/repos/{}/{}/actions/jobs/{job_id}/logs",
            self.owner, self.repo
        );
        let response = match self.client._get(log_path).await {
            Ok(response) => response,
            Err(error) => {
                diag::log_diag(&format!(
                    "tracker: get log for job {job_id} failed: {error}"
                ));
                return None;
            }
        };
        match response.into_body().collect().await {
            Ok(collected) => Some(String::from_utf8_lossy(&collected.to_bytes()).to_string()),
            Err(error) => {
                diag::log_diag(&format!(
                    "tracker: collect log body for job {job_id} failed: {error}"
                ));
                None
            }
        }
    }
}

fn job_failures(run_id: u64, job_name: &str, log_text: &str) -> Vec<ci::CiTestFailure> {
    let parsed = ci::parse_playwright_failures_from_log(run_id, log_text);
    if parsed.failures.is_empty() {
        return vec![ci::build_non_test_failure_record(job_name, log_text)];
    }
    parsed
        .failures
        .into_iter()
        .map(|mut failure| {
            failure.shard_id = Some(job_name.to_string());
            failure.stage = Some(job_name.to_string());
            failure
        })
        .collect()
}

fn workflow_failure_fallback(run_id: u64) -> ci::CiTestFailure {
    ci::CiTestFailure {
        shard_id: Some("(workflow)".to_string()),
        test_name: "(workflow failure: no job reached conclusion=failure)".to_string(),
        file: None,
        line: None,
        error_message: format!(
            "workflow run {run_id} conclusion=failure but no per-job failure-conclusion found. \
             Likely runner-infrastructure failure (cancelled / host died / pre-job setup). \
             Inspect GH Actions UI directly for this run."
        ),
        retry_count: 0,
        stage: Some("(workflow)".to_string()),
        class: Some("unknown_failure_class".to_string()),
    }
}
