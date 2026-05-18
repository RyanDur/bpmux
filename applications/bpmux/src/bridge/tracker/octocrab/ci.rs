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
                crate::session::diag::log_diag(&format!(
                    "tracker: fetch jobs for run {run_id} failed: {e}"
                ));
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
                crate::session::diag::log_diag(&format!(
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
                    crate::session::diag::log_diag(&format!(
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
    ) -> Result<crate::bridge::tools::CiRunFailures, crate::bridge::tools::CiLogError> {
        #[derive(serde::Deserialize)]
        struct Run {
            status: Option<String>,
            conclusion: Option<String>,
        }
        let run_path = format!("/repos/{}/{}/actions/runs/{run_id}", self.owner, self.repo);
        let run: Run = self.client.get(run_path, None::<&()>).await.map_err(|e| {
            let msg = format!("{e}");
            if msg.contains("404") {
                crate::bridge::tools::CiLogError::RunNotFound { run_id }
            } else {
                crate::bridge::tools::CiLogError::UpstreamError { detail: msg }
            }
        })?;
        let conclusion = run
            .conclusion
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if run.status.as_deref() != Some("completed") {
            return Err(crate::bridge::tools::CiLogError::RunInProgress {
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
            crate::bridge::tools::CiLogError::UpstreamError {
                detail: format!("fetch jobs for run {run_id}: {e}"),
            }
        })?;
        let mut all_failures: Vec<crate::bridge::tools::CiTestFailure> = Vec::new();
        for job in jobs.jobs {
            if job.conclusion.as_deref() != Some("failure") {
                continue;
            }
            let log_path = format!(
                "/repos/{}/{}/actions/jobs/{}/logs",
                self.owner, self.repo, job.id
            );
            use http_body_util::BodyExt;
            let log_text = match self.client._get(log_path).await {
                Ok(resp) => {
                    let body = resp.into_body().collect().await;
                    match body {
                        Ok(collected) => String::from_utf8_lossy(&collected.to_bytes()).to_string(),
                        Err(e) => {
                            crate::session::diag::log_diag(&format!(
                                "tracker: collect log body for job {} failed: {e}",
                                job.id
                            ));
                            continue;
                        }
                    }
                }
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "tracker: get log for job {} failed: {e}",
                        job.id
                    ));
                    continue;
                }
            };
            let parsed =
                crate::bridge::tools::parse_playwright_failures_from_log(run_id, &log_text);
            if parsed.failures.is_empty() {
                let fallback =
                    crate::bridge::tools::build_non_test_failure_record(&job.name, &log_text);
                all_failures.push(fallback);
            } else {
                for mut failure in parsed.failures {
                    failure.shard_id = Some(job.name.clone());
                    failure.stage = Some(job.name.clone());
                    all_failures.push(failure);
                }
            }
        }
        let final_failures = if all_failures.is_empty() && conclusion == "failure" {
            vec![crate::bridge::tools::CiTestFailure {
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
            }]
        } else {
            all_failures
        };
        Ok(crate::bridge::tools::CiRunFailures {
            run_id,
            conclusion,
            failures: final_failures,
        })
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
