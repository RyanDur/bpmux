use crate::bridge::tracker::WorkflowRunEntry;
use crate::github;

pub struct GitHubApi {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
}

impl GitHubApi {
    pub fn from_env() -> Option<Self> {
        match crate::bridge::tracker::select_from_env(|k| std::env::var(k).ok()) {
            crate::bridge::tracker::TrackerSelection::Octocrab { owner, repo, token } => {
                github::ensure_crypto_provider_installed();
                let client = octocrab::OctocrabBuilder::new()
                    .personal_token(token)
                    .build()
                    .ok()?;
                Some(Self {
                    client,
                    owner,
                    repo,
                })
            }
            crate::bridge::tracker::TrackerSelection::Fallback(reason) => {
                crate::session::diag::log_diag(&format!(
                    "GitHubApi::from_env: missing env config ({reason:?})"
                ));
                None
            }
        }
    }

    pub async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
        #[derive(serde::Deserialize)]
        struct RunsResponse {
            workflow_runs: Vec<WorkflowRunEntry>,
        }
        let path = format!(
            "/repos/{}/{}/actions/runs?branch=main&per_page=1",
            self.owner, self.repo
        );
        let response: RunsResponse = self
            .client
            .get(path, None::<&()>)
            .await
            .map_err(Self::classify)?;
        let Some(mut latest) = response.workflow_runs.into_iter().next() else {
            return Ok(None);
        };
        if latest.conclusion.as_deref() == Some("failure")
            && let Some(run_id) = latest.database_id
        {
            latest.annotation_message = self.fetch_first_failure_annotation(run_id).await;
        }
        Ok(Some(latest))
    }

    async fn fetch_first_failure_annotation(&self, run_id: u64) -> Option<String> {
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
                    "github_api: fetch jobs for run {run_id} failed: {e}"
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
                    "github_api: fetch check-run failed for run {run_id}: {e}"
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
                        "github_api: fetch annotations failed for run {run_id}: {e}"
                    ));
                    return None;
                }
            };
        annotations.into_iter().find_map(|a| a.message)
    }

    fn classify(err: octocrab::Error) -> String {
        github::classify_error(&err).user_message()
    }
}
