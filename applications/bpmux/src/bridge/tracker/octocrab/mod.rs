mod ci;
mod conditional;
mod json_shape;
mod rest_helpers;
mod trait_impl;

use json_shape::issue_to_board_json;

use super::trait_def::{
    ConditionalIssue, ConditionalRunEntry, ensure_crypto_provider_installed,
    log_labels_fetch_failure,
};

#[derive(Clone)]
pub struct OctocrabStoryTracker {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
    rate_limit_observer: crate::github::RateLimitObserver,
}

impl OctocrabStoryTracker {
    pub fn rate_limit_observer(&self) -> crate::github::RateLimitObserver {
        self.rate_limit_observer.clone()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TrackerSelection {
    Octocrab {
        owner: String,
        repo: String,
        token: String,
    },
    OctocrabWithBaseUrl {
        owner: String,
        repo: String,
        base_url: String,
    },
    Fallback(FallbackReason),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FallbackReason {
    NoRepo,
    MalformedRepo,
    NoToken,
}

pub(crate) fn select_from_env<F>(get: F) -> TrackerSelection
where
    F: Fn(&str) -> Option<String>,
{
    let Some(repo_spec) = get("BPMUX_REPO") else {
        return TrackerSelection::Fallback(FallbackReason::NoRepo);
    };
    let Some((owner, repo)) = repo_spec.split_once('/') else {
        return TrackerSelection::Fallback(FallbackReason::MalformedRepo);
    };
    if owner.is_empty() || repo.is_empty() {
        return TrackerSelection::Fallback(FallbackReason::MalformedRepo);
    }
    if let Some(base_url) = get("BPMUX_GH_BASE_URL") {
        return TrackerSelection::OctocrabWithBaseUrl {
            owner: owner.to_string(),
            repo: repo.to_string(),
            base_url,
        };
    }
    let Some(token) = get("GH_TOKEN").or_else(|| get("GITHUB_TOKEN")) else {
        return TrackerSelection::Fallback(FallbackReason::NoToken);
    };
    TrackerSelection::Octocrab {
        owner: owner.to_string(),
        repo: repo.to_string(),
        token,
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
                    rate_limit_observer: crate::github::RateLimitObserver::new(),
                })
            }
            TrackerSelection::OctocrabWithBaseUrl {
                owner,
                repo,
                base_url,
            } => match Self::with_base_url(&base_url, owner, repo) {
                Ok(tracker) => Some(tracker),
                Err(e) => {
                    crate::session::diag::log_diag(&format!(
                        "OctocrabStoryTracker::from_env: BPMUX_GH_BASE_URL build failed: {e}"
                    ));
                    None
                }
            },
            TrackerSelection::Fallback(reason) => {
                crate::session::diag::log_diag(&format!(
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
            rate_limit_observer: crate::github::RateLimitObserver::new(),
        })
    }

    fn classify(err: octocrab::Error) -> String {
        crate::github::classify_error(&err).user_message()
    }

    async fn get_with_capture<R: serde::de::DeserializeOwned>(
        &self,
        path: String,
    ) -> Result<R, String> {
        use http_body_util::BodyExt;
        let response = self.client._get(path).await.map_err(Self::classify)?;
        crate::github::rate_limit::capture_headers(
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

    pub async fn get_story_conditional(
        &self,
        number: u64,
        prior_etag: Option<&str>,
    ) -> Result<ConditionalIssue, String> {
        use http_body_util::BodyExt;
        let path = format!("/repos/{}/{}/issues/{number}", self.owner, self.repo);
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
        crate::github::rate_limit::capture_headers(
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
            return Ok(ConditionalIssue::NotModified);
        }
        if status.as_u16() == 404 {
            return Ok(ConditionalIssue::NotFound);
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
        let issue: octocrab::models::issues::Issue =
            serde_json::from_slice(&body).map_err(|e| format!("json parse: {e}"))?;
        Ok(ConditionalIssue::Modified {
            value: issue_to_board_json(&issue),
            etag,
        })
    }

    pub async fn fetch_board_stories_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let since_q = since.map(|s| {
            format!(
                "&since={}",
                s.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            )
        });
        let mut all_issues: Vec<octocrab::models::issues::Issue> = Vec::new();
        let mut page = 1u32;
        loop {
            let path = format!(
                "/repos/{}/{}/issues?state=open&per_page=100&page={}{}",
                self.owner,
                self.repo,
                page,
                since_q.as_deref().unwrap_or("")
            );
            let issues: Vec<octocrab::models::issues::Issue> = self.get_with_capture(path).await?;
            let count = issues.len();
            all_issues.extend(issues);
            if count < 100 {
                break;
            }
            page += 1;
        }
        let open_count = all_issues.len();
        let mut closed_count = 0usize;
        let mut closed_page = 1u32;
        loop {
            let closed_path = format!(
                "/repos/{}/{}/issues?state=closed&per_page=100&sort=updated&direction=desc&page={}{}",
                self.owner,
                self.repo,
                closed_page,
                since_q.as_deref().unwrap_or("")
            );
            let closed: Vec<octocrab::models::issues::Issue> =
                self.get_with_capture(closed_path).await?;
            let count = closed.len();
            all_issues.extend(closed);
            closed_count += count;
            if count < 100 {
                break;
            }
            closed_page += 1;
        }
        crate::session::diag::log_diag(&format!(
            "github fetch_board_stories_since(since={:?}): {open_count} open (paginated) + {closed_count} closed (paginated) = {} total",
            since.map(|s| s.to_rfc3339()),
            all_issues.len()
        ));
        Ok(all_issues.iter().map(issue_to_board_json).collect())
    }
}

#[cfg(test)]
mod gh_state_label_tests {
    use super::json_shape::gh_state_label;
    use octocrab::models::IssueState;

    #[test]
    fn gh_state_label_maps_open_and_closed() {
        assert_eq!(gh_state_label(&IssueState::Open), "open");
        assert_eq!(gh_state_label(&IssueState::Closed), "closed");
    }
}
