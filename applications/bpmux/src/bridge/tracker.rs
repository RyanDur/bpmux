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
    /// #397: annotation message from the check-run surfaced on
    /// failed runs so classification can distinguish quota-
    /// exhausted / billing-blocked (pass-with-citation) from genuine
    /// test/build failures (hard-block). Populated by
    /// `list_latest_run_on_main` for failed runs; `None` for
    /// successful runs (annotation fetch skipped).
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

pub trait StoryTracker: Clone + Send + Sync + 'static {
    /// #1151: returns the post-mutation Issue JSON so the adapter can
    /// parse it into a `Story` and dispatch `BoardEvent::StoryRefreshed`
    /// without a follow-up `refresh_story(N)` call. octocrab's
    /// `issues().update(N).state(Closed).send()` returns the updated
    /// Issue; we capture it here instead of discarding via `.map(|_|
    /// String)`.
    fn close_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-mutation Issue JSON (parallel to
    /// `close_story`).
    fn reopen_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-mutation labels list (octocrab's
    /// `add_labels` returns `Vec<Label>`). Capturing the labels list
    /// lets callers re-derive the local Story state via
    /// `state_from_labels` without a follow-up `get_story` round-trip.
    fn add_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    /// #1151: returns the post-mutation labels list. Empty Vec on the
    /// 404-idempotent path (label already absent).
    fn remove_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<Vec<String>, String>> + Send;
    /// #1151: returns the freshly-created Issue JSON. Includes the
    /// auto-assigned issue number; callers that previously parsed
    /// the format-string `"#{number} {title}"` should read
    /// `value["number"]` directly.
    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #1151: returns the post-update Issue JSON.
    fn update_body(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<serde_json::Value, String>> + Send;
    /// #403: full board as typed JSON Values — used by `list_ready`
    /// and other filtered enumerations. Counterpart to the inherent
    /// `OctocrabStoryTracker::list_stories` (returns pre-serialized
    /// string); keeping both so callers that want the raw string
    /// (legacy contract tests) don't pay a re-serialize round-trip.
    fn fetch_board_stories(
        &self,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send;
    /// #403: full detail for one story. Returns `Ok(None)` when the
    /// issue doesn't exist so handlers can map to a typed "not
    /// found" response instead of erroring on an expected absence.
    fn get_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Option<serde_json::Value>, String>> + Send;
    fn comment_on_story(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn list_story_comments(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<Vec<serde_json::Value>, String>> + Send;
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalIssue {
    NotModified,
    Modified {
        value: serde_json::Value,
        etag: Option<String>,
    },
    NotFound,
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
        // Shared GitHub-error taxonomy lives in `crate::github` so
        // the bridge and the board poller both depend on it instead
        // of one reaching across into the other's layer.
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
        let closed_path = format!(
            "/repos/{}/{}/issues?state=closed&per_page=100&sort=updated&direction=desc{}",
            self.owner,
            self.repo,
            since_q.as_deref().unwrap_or("")
        );
        let closed: Vec<octocrab::models::issues::Issue> =
            self.get_with_capture(closed_path).await?;
        let closed_count = closed.len();
        all_issues.extend(closed);
        crate::session::diag::log_diag(&format!(
            "github fetch_board_stories_since(since={:?}): {open_count} open (paginated) + {closed_count} closed (1-page most-recent) = {} total",
            since.map(|s| s.to_rfc3339()),
            all_issues.len()
        ));
        Ok(all_issues.iter().map(issue_to_board_json).collect())
    }

    // `fetch_board_stories` + `get_story` moved to the
    // `StoryTracker` trait impl for this type (#403 — list_ready
    // + get_story MCP handlers take `&impl StoryTracker` so they
    // need the methods trait-visible). The impls themselves are
    // unchanged; they just live inside the trait block now.

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

    /// Fetch the issue's timeline: creation (as a "filed" event) plus
    /// every GitHub issue event (labeled / unlabeled / closed / reopened
    /// / commented / ...) in chronological order. Returns a condensed
    /// JSON array — the renderer classifies events into lifecycle gates
    /// (started, delivered, accepted, rejected, closed) from the raw
    /// event + label combination. Serialisable through serde_json so
    /// the REST handler can forward without re-deriving.
    pub async fn list_story_timeline(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        // First: the issue itself, for the "filed" anchor event.
        let issue = self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .map_err(Self::classify)?;
        let mut out: Vec<serde_json::Value> = vec![serde_json::json!({
            "event": "filed",
            "actor": issue.user.login,
            "ts": issue.created_at.to_rfc3339(),
        })];

        // Timeline events: labels, closed, reopened, commented, etc.
        let timeline = self
            .client
            .issues(&self.owner, &self.repo)
            .list_timeline_events(number)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        for e in timeline.items {
            // Not every timeline entry has a created_at (committed-
            // event-only variants expose `submitted_at` / commit SHA
            // instead). Skip anything we can't time-order — those
            // aren't lifecycle-relevant for the chat-tool timeline
            // anyway.
            let Some(ts) = e.created_at else { continue };
            let actor = e
                .actor
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_default();
            let event = serde_json::to_value(&e.event)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            let mut entry = serde_json::json!({
                "event": event,
                "actor": actor,
                "ts": ts.to_rfc3339(),
            });
            // Label payload comes through as { name, color } — the
            // renderer keys lifecycle classification off the label
            // name, so surface just the name rather than the full
            // label object.
            if let Some(label) = &e.label
                && let Some(entry_obj) = entry.as_object_mut()
            {
                entry_obj.insert(
                    "label".into(),
                    serde_json::Value::String(label.name.clone()),
                );
            }
            // Commented events carry the comment body so the timeline
            // can show a short excerpt inline (spec's "optional note"
            // column).
            if let Some(body) = &e.body
                && let Some(entry_obj) = entry.as_object_mut()
            {
                entry_obj.insert("body".into(), serde_json::Value::String(body.clone()));
            }
            out.push(entry);
        }
        // Timeline API returns chronologically already; sorting
        // defensively so the "filed" synthetic entry always leads
        // the array even if issue.created_at matches an event ts
        // (unlikely but safer than trusting API ordering).
        out.sort_by(|a, b| {
            let sa = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            let sb = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            sa.cmp(sb)
        });
        Ok(out)
    }

    /// Post a comment on a story. Thin wrapper around the trait method
    /// for the REST handler's convenience (keeps the REST layer off
    /// the StoryTracker trait so the FakeTracker doesn't need to stub
    /// a new method).
    pub async fn post_story_comment(&self, number: u64, body: &str) -> Result<(), String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }

    pub async fn list_stories(&self) -> Result<String, String> {
        let first = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        let issues = self.client.all_pages(first).await.map_err(Self::classify)?;
        let condensed: Vec<serde_json::Value> = issues
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

    pub async fn get_story_title(&self, number: u64) -> Option<String> {
        self.client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .ok()
            .map(|i| i.title)
    }
}

/// Shape a single octocrab Issue into a raw-JSON envelope used by
/// `fetch_board_stories` + `get_story` for adapter ingest. The decoder
/// `board::adapters::github::decode_story_metadata` reads `gh_state`
/// and `labels` and derives canonical `Story.state` via the single
/// classifier `state_from_labels` per Ryan's 2026-04-27 boundary
/// principle ruling. This function is the boundary OUT — it serializes
/// raw GH issue facts; no state classification happens here.
fn gh_state_label(state: &octocrab::models::IssueState) -> &'static str {
    match state {
        octocrab::models::IssueState::Open => "open",
        octocrab::models::IssueState::Closed => "closed",
        _ => "unknown",
    }
}

fn issue_to_board_json(issue: &octocrab::models::issues::Issue) -> serde_json::Value {
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    let is_closed = matches!(issue.state, octocrab::models::IssueState::Closed);
    let state = crate::board::adapters::github::state_from_labels(&labels, is_closed).as_str();
    let gh_state = gh_state_label(&issue.state);
    let parent_epic = crate::board::adapters::github::parent_epic_from_labels(&labels);
    serde_json::json!({
        "number": issue.number,
        "title": issue.title,
        "state": state,
        "gh_state": gh_state,
        "labels": labels,
        "parent_epic": parent_epic,
        "assignees": issue.assignees.iter().map(|a| a.login.clone()).collect::<Vec<_>>(),
        "created_at": issue.created_at.to_rfc3339(),
        "body": issue.body.clone().unwrap_or_default(),
    })
}

impl StoryTracker for OctocrabStoryTracker {
    async fn close_story(&self, number: u64) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn reopen_story(&self, number: u64) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(octocrab::models::IssueState::Open)
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<Vec<String>, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .add_labels(number, &[label.to_string()])
            .await
            .map(|labels| labels.into_iter().map(|l| l.name).collect())
            .map_err(Self::classify)
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<Vec<String>, String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, label)
            .await
        {
            Ok(labels) => Ok(labels.into_iter().map(|l| l.name).collect()),
            // 404 on remove_label means the label wasn't attached —
            // treat as a successful no-op. Matches gh's lenient
            // behavior for --remove-label on an unlabeled issue.
            // Empty Vec is the post-state we'd report; callers
            // re-derive state from this list, so empty is safe.
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(Vec::new())
            }
            Err(e) => Err(Self::classify(e)),
        }
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<serde_json::Value, String> {
        let issues = self.client.issues(&self.owner, &self.repo);
        let mut builder = issues.create(title);
        if !body.is_empty() {
            builder = builder.body(body);
        }
        builder
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn update_body(&self, number: u64, body: &str) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .body(body)
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        self.fetch_board_stories_since(None).await
    }

    /// #403: fetch one story's full detail. Returns `Ok(None)` when
    /// the issue doesn't exist (GitHub 404) so handlers can map to
    /// a typed not-found response; `Err` for any other failure.
    async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
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

    async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| format!("#{number} commented"))
            .map_err(Self::classify)
    }

    async fn list_story_comments(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        let page = self
            .client
            .issues(&self.owner, &self.repo)
            .list_comments(number)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        let mut condensed: Vec<serde_json::Value> = page
            .items
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.into_inner(),
                    "author": c.user.login,
                    "body": c.body.unwrap_or_default(),
                    "created_at": c.created_at.to_rfc3339(),
                })
            })
            .collect();
        condensed.sort_by(|a, b| {
            let ak = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            let bk = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            ak.cmp(bk)
        });
        Ok(condensed)
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
        let response: RunsResponse = self.get_with_capture(path).await?;
        let Some(mut latest) = response.workflow_runs.into_iter().next() else {
            return Ok(None);
        };
        // #397: on failed runs, fetch the first annotation message so
        // `classify_runs` can distinguish quota-exhausted (pass-with-
        // citation at team gates) from code-failure (hard-block).
        // Skipped for successful runs — no distinction needed, avoids
        // the extra API calls on the happy path.
        if latest.conclusion.as_deref() == Some("failure")
            && let Some(run_id) = latest.database_id
        {
            latest.annotation_message = self.fetch_first_failure_annotation(run_id).await;
        }
        Ok(Some(latest))
    }
}

// Inherent impl for OctocrabStoryTracker (separate from the trait
// impl above) — houses helper methods that aren't part of the
// StoryTracker trait surface but are internal to the octocrab-
// backed tracker.
impl OctocrabStoryTracker {
    /// Fetch the first annotation message for the primary check-run
    /// of a failed workflow run. Returns None on any error path
    /// (logged via diag) — annotations are advisory for
    /// classification, not load-bearing for functionality. Worst-
    /// case: we miss the quota-exhausted distinction and classify
    /// as generic Red, which is the pre-#397 behavior.
    ///
    /// Two HTTP calls per failed run:
    ///   1. /actions/runs/{id}/jobs → primary job's check_run_url
    ///   2. GET that check_run_url → output.annotations_url → fetch
    ///      first annotation message
    ///
    /// Cost budget: only fires on failure path, which is transient
    /// in healthy state; GitHub rate limit (5000/hr authed) has
    /// substantial headroom for realistic polling cadences.
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
}

#[cfg(test)]
mod gh_state_label_tests {
    use super::*;
    use octocrab::models::IssueState;

    #[test]
    fn gh_state_label_maps_open_and_closed() {
        assert_eq!(gh_state_label(&IssueState::Open), "open");
        assert_eq!(gh_state_label(&IssueState::Closed), "closed");
    }
}
