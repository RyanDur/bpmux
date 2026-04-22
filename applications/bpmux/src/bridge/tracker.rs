/// Re-export so existing `super::tracker::ensure_crypto_provider_installed`
/// imports from sibling modules keep resolving. The real implementation
/// moved to `crate::github::client` during #320 so terminal-side fetchers
/// could reuse the same idempotent install without reaching across layers.
pub(super) use crate::github::ensure_crypto_provider_installed;

/// Shared fail-soft handler for `get_story_labels`: log the error
/// through the diag sink so it's observable via `BPMUX_LOG`, then
/// return an empty Vec. Caller treats empty as "ambiguous category"
/// and defaults the addressee up to Ryan (classifier fail-safe).
///
/// Factored out so both tracker impls emit the same diagnostic text
/// and unit tests can assert the observable behavior without needing
/// live gh/octocrab.
pub(crate) fn log_labels_fetch_failure<E: std::fmt::Display>(
    backend: &str,
    number: u64,
    err: E,
) -> Vec<String> {
    crate::session::diag::log_diag(&format!(
        "get_story_labels ({backend}) #{number} failed: {err}"
    ));
    Vec::new()
}

/// Lightweight representation of a single workflow run on GitHub
/// Actions — what `get_ci_state` returns and what `terminal::ci`
/// classifies. Single source of truth so the TUI poller + the MCP
/// tool share the same vocabulary (#320).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct WorkflowRunEntry {
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "head_sha", alias = "headSha", default)]
    pub head_sha: Option<String>,
    #[serde(rename = "display_title", alias = "displayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "id", alias = "databaseId", default)]
    pub database_id: Option<u64>,
}

pub trait StoryTracker: Clone + Send + Sync + 'static {
    fn close_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn add_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn remove_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn list_stories(&self) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn comment_on_story(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn get_story_title(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
    /// Fetch labels for story-category routing (pass_desk_check
    /// signal picks addressee based on honest-plumbing vs vertical).
    /// Returns empty Vec when the story can't be fetched — caller
    /// treats empty as "ambiguous category" and defaults up.
    fn get_story_labels(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Vec<String>> + Send;
    /// Fetch the most recent workflow run on main (#302 / #320).
    /// Returns `Ok(None)` when the repo has no runs yet. Errors
    /// surface as `Err(String)` to match the rest of the trait's
    /// Result-convention.
    fn list_latest_run_on_main(
        &self,
    ) -> impl std::future::Future<Output = Result<Option<WorkflowRunEntry>, String>> + Send;
}

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
}

/// Result of inspecting env vars to decide whether we can build an
/// `OctocrabStoryTracker`. Separates env-decisioning (pure, testable)
/// from the octocrab client build (impure, requires network config).
/// `Octocrab` is a successful selection; `Fallback` names WHY the
/// bridge startup will fail-loud (#320: no more subprocess fallback).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TrackerSelection {
    Octocrab {
        owner: String,
        repo: String,
        token: String,
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
                })
            }
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
    /// the mock without any real-network side effects. Not used at
    /// runtime — the production path always goes through `from_env`.
    #[cfg(test)]
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
        })
    }

    fn classify(err: octocrab::Error) -> String {
        // Shared GitHub-error taxonomy lives in `crate::github` so
        // the bridge and the board poller both depend on it instead
        // of one reaching across into the other's layer.
        crate::github::classify_error(&err).user_message()
    }

    /// Fetch the full board: all stories with the fields the Electron
    /// renderer needs (number, title, state, labels, assignees,
    /// created_at, body). Wraps the same underlying octocrab `list`
    /// call `list_stories` uses, but returns a richer serde_json
    /// payload. Used by the HTTP `/api/board` endpoint.
    pub async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        let page = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        Ok(page.items.iter().map(issue_to_board_json).collect())
    }

    /// Fetch a single story with full body text. Returns `Ok(None)`
    /// when the issue doesn't exist (GitHub 404); `Err` for any other
    /// failure so the REST handler can map to 503.
    pub async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
        {
            Ok(issue) => Ok(Some(issue_to_board_json(&issue))),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(None)
            }
            Err(e) => Err(Self::classify(e)),
        }
    }

    /// Accept a delivered story: remove the `delivered` label, close
    /// the issue. Mirrors the label/close sequence in
    /// `tools_stories::handle_accept_story` but without the ws signal
    /// side effects — the REST handler fires the board-invalidate
    /// broadcast itself via the session.
    pub async fn accept_story(&self, number: u64) -> Result<(), String> {
        // Best-effort label removal: a 404 means the label wasn't
        // attached, matching the MCP path's tolerance.
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, "delivered")
            .await
        {
            match e {
                octocrab::Error::GitHub { ref source, .. }
                    if source.status_code.as_u16() == 404 => {}
                other => {
                    crate::session::diag::log_diag(&format!(
                        "rest: accept_story remove-label delivered failed for #{number}: {}",
                        Self::classify(other)
                    ));
                }
            }
        }
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }

    /// Reject a delivered story: remove `delivered`, add `rejected`,
    /// post the rejection note as a comment. Returns Err only when the
    /// load-bearing comment post fails — label updates are best-effort
    /// so a stale label doesn't hide a real rejection from the team.
    pub async fn reject_story(&self, number: u64, note: &str) -> Result<(), String> {
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, "delivered")
            .await
        {
            match e {
                octocrab::Error::GitHub { ref source, .. }
                    if source.status_code.as_u16() == 404 => {}
                other => {
                    crate::session::diag::log_diag(&format!(
                        "rest: reject_story remove-label delivered failed for #{number}: {}",
                        Self::classify(other)
                    ));
                }
            }
        }
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .add_labels(number, &["rejected".to_string()])
            .await
        {
            crate::session::diag::log_diag(&format!(
                "rest: reject_story add-label rejected failed for #{number}: {}",
                Self::classify(e)
            ));
        }
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, note)
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }
}

/// Shape a single octocrab Issue into the board JSON envelope the
/// Electron renderer expects. Lifted out as a free function so
/// `fetch_board_stories` and `get_story` emit identical shapes — the
/// renderer can reuse the same parser for both endpoints.
fn issue_to_board_json(issue: &octocrab::models::issues::Issue) -> serde_json::Value {
    serde_json::json!({
        "number": issue.number,
        "title": issue.title,
        "state": match issue.state {
            octocrab::models::IssueState::Open => "OPEN",
            octocrab::models::IssueState::Closed => "CLOSED",
            _ => "OPEN",
        },
        "labels": issue.labels.iter().map(|l| l.name.clone()).collect::<Vec<_>>(),
        "assignees": issue.assignees.iter().map(|a| a.login.clone()).collect::<Vec<_>>(),
        "created_at": issue.created_at.to_rfc3339(),
        "body": issue.body.clone().unwrap_or_default(),
    })
}

impl StoryTracker for OctocrabStoryTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await
            .map(|_| format!("#{number} closed"))
            .map_err(Self::classify)
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .add_labels(number, &[label.to_string()])
            .await
            .map(|_| format!("#{number} labeled '{label}'"))
            .map_err(Self::classify)
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, label)
            .await
        {
            Ok(_) => Ok(format!("#{number} unlabeled '{label}'")),
            // 404 on remove_label means the label wasn't attached —
            // treat as a successful no-op. Matches gh's lenient
            // behavior for --remove-label on an unlabeled issue.
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(format!("#{number} label '{label}' already absent"))
            }
            Err(e) => Err(Self::classify(e)),
        }
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<String, String> {
        let issues = self.client.issues(&self.owner, &self.repo);
        let mut builder = issues.create(title);
        if !body.is_empty() {
            builder = builder.body(body);
        }
        builder
            .send()
            .await
            .map(|issue| format!("#{} {}", issue.number, issue.title))
            .map_err(Self::classify)
    }

    async fn list_stories(&self) -> Result<String, String> {
        let page = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        // Shape matches what the existing callers parse via serde_json::Value
        // (fields: number, title, state, labels). Emit as a JSON array so the
        // caller's serde_json::from_str continues to work unchanged.
        let condensed: Vec<serde_json::Value> = page
            .items
            .iter()
            .map(|i| {
                serde_json::json!({
                    "number": i.number,
                    "title": i.title,
                    "state": match i.state {
                        octocrab::models::IssueState::Open => "OPEN",
                        octocrab::models::IssueState::Closed => "CLOSED",
                        _ => "OPEN",
                    },
                    "labels": i.labels.iter().map(|l| serde_json::json!({"name": l.name})).collect::<Vec<_>>(),
                })
            })
            .collect();
        serde_json::to_string(&condensed).map_err(|e| format!("serialize: {e}"))
    }

    async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| format!("#{number} commented"))
            .map_err(Self::classify)
    }

    async fn get_story_title(&self, number: u64) -> Option<String> {
        self.client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .ok()
            .map(|i| i.title)
    }

    async fn get_story_labels(&self, number: u64) -> Vec<String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
        {
            Ok(i) => i.labels.iter().map(|l| l.name.clone()).collect(),
            Err(e) => log_labels_fetch_failure("octocrab", number, Self::classify(e)),
        }
    }

    async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
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
        Ok(response.workflow_runs.into_iter().next())
    }
}

#[derive(Clone, Default)]
pub struct FakeTracker {
    pub board_fixture: Option<String>,
}

impl StoryTracker for FakeTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        Ok(format!("(fake) #{number} closed"))
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} labeled '{label}'"))
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} unlabeled '{label}'"))
    }

    async fn create_story(&self, title: &str, _body: &str) -> Result<String, String> {
        Ok(format!("(fake) created: {title}"))
    }

    async fn list_stories(&self) -> Result<String, String> {
        if let Some(ref path) = self.board_fixture {
            return std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"));
        }
        Ok("[]".to_string())
    }

    async fn comment_on_story(&self, number: u64, _body: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} commented"))
    }

    async fn get_story_title(&self, number: u64) -> Option<String> {
        Some(format!("(fake) story #{number}"))
    }

    async fn get_story_labels(&self, _number: u64) -> Vec<String> {
        Vec::new()
    }

    async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
        Ok(None)
    }
}
