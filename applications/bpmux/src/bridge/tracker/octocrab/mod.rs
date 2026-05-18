//! Octocrab-backed `StoryTracker` implementation + sister tracker
//! plumbing. The sole production tracker post-#320 (the previous
//! gh-CLI subprocess fallback was retired); the bridge fails-loud
//! at startup when env config is insufficient instead of falling
//! back to subprocess.
//!
//! Decomposed in #1416 Slice 2 along axis-of-change boundaries:
//! - this `mod.rs` holds the `OctocrabStoryTracker` struct +
//!   env-decisioning + REST mutation helpers + the
//!   `impl StoryTracker` block;
//! - sister `conditional` holds ETag-aware fetch methods;
//! - sister `ci` holds GH-Actions CI-axis methods
//!   (annotations, failed-test extraction, rerun).

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

/// Octocrab-backed StoryTracker. Issues HTTP requests directly
/// to GitHub's REST API. Sole production tracker — the former
/// `GhTracker` subprocess fallback was retired in #320 to unify
/// on a single backend + drop gh CLI runtime dependency.
///
/// Constructed from an authenticated `Octocrab` client + owner/repo
/// strings. Production path builds it from `GH_TOKEN`/`GITHUB_TOKEN`
/// + `BPMUX_REPO` env vars; see `OctocrabStoryTracker::from_env`.
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

/// Result of inspecting env vars to decide whether we can build an
/// `OctocrabStoryTracker`. Separates env-decisioning (pure, testable)
/// from the octocrab client build (impure, requires network config).
/// `Octocrab` is a successful selection; `Fallback` names WHY the
/// bridge startup will fail-loud (#320: no more subprocess fallback).
///
/// `OctocrabWithBaseUrl` (#482) is the cross-bridge-substrate path:
/// when `BPMUX_GH_BASE_URL` is set the bridge subprocess points at
/// a test-controlled MockServer instead of api.github.com, enabling
/// multiple bridges to share a single mock GitHub source-of-truth
/// for cross-bridge mutation-propagation tests. No GH_TOKEN required
/// since httpmock doesn't validate auth.
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

/// Named reasons `OctocrabStoryTracker::from_env` returns `None`.
/// Pre-#320 these were "fall back to gh CLI" branches; post-#320
/// they're "fail-loud the bridge can't start" — the enum name +
/// variant names stuck for diff continuity with #298. Diag logs
/// cite the specific reason so the user sees WHICH env var is
/// missing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FallbackReason {
    /// `BPMUX_REPO` env var missing entirely.
    NoRepo,
    /// `BPMUX_REPO` present but doesn't match "owner/name" format
    /// (no `/` separator or empty segments).
    MalformedRepo,
    /// Neither `GH_TOKEN` nor `GITHUB_TOKEN` set. Octocrab can't
    /// auth without one.
    NoToken,
}

/// Pure env-decisioning. Accepts a closure reader so tests can
/// inject arbitrary env-var maps without mutating the process env
/// (which would require a global lock for parallel test safety —
/// existing `tracker_tests.rs` uses that pattern for log-diag,
/// but for pure selection-logic a closure is lighter).
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
    /// Build from env vars. Returns None when required auth/repo
    /// config is missing — bridge startup (run_bridge) fails loud in
    /// that case (#320; pre-#320 this was a fallback path).
    ///
    /// Logs the specific missing-env reason to diag on None so the
    /// user sees WHICH var needs setting (NoRepo / MalformedRepo /
    /// NoToken). Pattern preserved from #298's silent-swap diagnosis.
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

    /// Build a tracker pointed at an arbitrary base URL instead of
    /// `https://api.github.com`. Contract tests spin up an httpmock
    /// server and pass its URL here so the tracker's HTTP calls hit
    /// the mock without any real-network side effects. Also reachable
    /// from production via `from_env` when `BPMUX_GH_BASE_URL` is set
    /// (#482 cross-bridge substrate); the runtime path stays
    /// api.github.com when the env var is absent.
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
