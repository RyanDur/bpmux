mod ci_failures;
mod ci_jobs;
mod conditional;
mod errors;
mod fetch;
mod json_shape;
mod rest_helpers;
mod selection;
mod sub_issues;
mod trait_impl;

#[cfg(test)]
mod gh_state_label_tests;

#[cfg(test)]
pub(crate) use selection::FallbackReason;
pub(crate) use selection::{TrackerSelection, select_from_env};

use super::trait_def::{
    ConditionalRunEntry, ensure_crypto_provider_installed, log_labels_fetch_failure,
};

#[derive(Clone)]
pub struct OctocrabStoryTracker {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
    rate_limit_observer: crate::rate_limit::RateLimitObserver,
}

impl OctocrabStoryTracker {
    pub fn rate_limit_observer(&self) -> crate::rate_limit::RateLimitObserver {
        self.rate_limit_observer.clone()
    }
}

impl OctocrabStoryTracker {
    pub fn from_env() -> Option<Self> {
        match select_from_env(|k| std::env::var(k).ok()) {
            TrackerSelection::Octocrab { owner, repo, token } => {
                ensure_crypto_provider_installed();
                let client = octocrab::OctocrabBuilder::new()
                    .personal_token(token)
                    .build()
                    .ok()?;
                Some(Self {
                    client,
                    owner,
                    repo,
                    rate_limit_observer: crate::rate_limit::RateLimitObserver::new(),
                })
            }
            TrackerSelection::OctocrabWithBaseUrl {
                owner,
                repo,
                base_url,
            } => match Self::with_base_url(&base_url, owner, repo) {
                Ok(tracker) => Some(tracker),
                Err(e) => {
                    diag::log_diag(&format!(
                        "OctocrabStoryTracker::from_env: BPMUX_GH_BASE_URL build failed: {e}"
                    ));
                    None
                }
            },
            TrackerSelection::Fallback(reason) => {
                diag::log_diag(&format!(
                    "OctocrabStoryTracker::from_env: missing env config ({reason:?})"
                ));
                None
            }
        }
    }

    pub(crate) fn with_base_url(
        base_url: &str,
        owner: String,
        repo: String,
    ) -> Result<Self, String> {
        ensure_crypto_provider_installed();
        let client = octocrab::OctocrabBuilder::new()
            .base_uri(base_url)
            .map_err(|e| format!("base_uri: {e}"))?
            .build()
            .map_err(|e| format!("build: {e}"))?;
        Ok(Self {
            client,
            owner,
            repo,
            rate_limit_observer: crate::rate_limit::RateLimitObserver::new(),
        })
    }

    fn classify(err: octocrab::Error) -> String {
        errors::classify_error(&err).user_message()
    }

    async fn set_issue_state(
        &self,
        number: u64,
        state: octocrab::models::IssueState,
    ) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(state)
            .send()
            .await
            .map(|issue| json_shape::issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn get_with_capture<R: serde::de::DeserializeOwned>(
        &self,
        path: String,
    ) -> Result<R, String> {
        use http_body_util::BodyExt;
        let response = self.client._get(path).await.map_err(Self::classify)?;
        crate::rate_limit::capture_headers(
            response.headers(),
            &self.rate_limit_observer,
            std::time::SystemTime::now(),
        );
        let status = response.status();
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
        serde_json::from_slice(&body).map_err(|e| format!("json parse: {e}"))
    }
}
