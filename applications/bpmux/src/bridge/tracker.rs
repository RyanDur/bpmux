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
    /// #403: full board as typed JSON Values — used by `list_ready`
    /// and other filtered enumerations. Counterpart to
    /// `list_stories` (which returns the pre-serialized string);
    /// keeping both so callers that want the raw string (TUI's
    /// view_board) don't pay a re-serialize round-trip.
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
}

/// Derive the renderer's board-bucket state string from a GitHub
/// issue's raw state + labels. Mirrors the label-precedence logic in
/// `terminal::board::stories::parse_issue_values` (same order, same
/// fallbacks) but emits the kebab-case strings the Electron board
/// grouper expects — see `applications/electron/src/renderer/src/board/types.ts`.
///
/// Duplication flag: this label→state mapping also lives in
/// `terminal/board/stories.rs`. They must stay in sync; diverging
/// produces the exact silent-drop bug that stalled #xxx acceptance
/// walk (empty buckets on live data, no error).
fn derive_renderer_state(
    gh_state: &octocrab::models::IssueState,
    labels: &[String],
) -> &'static str {
    if matches!(gh_state, octocrab::models::IssueState::Closed) {
        return "accepted";
    }
    let has = |name: &str| labels.iter().any(|l| l == name);
    if has("delivered") {
        "delivered"
    } else if has("in progress") || has("started") || has("finished") {
        // LABEL_IN_PROGRESS ("in progress" with a space) is what
        // start_story applies; "started"/"finished" are legacy TUI
        // labels. Matching all three keeps cross-tool compatibility
        // — TUI-originated + Electron-originated + MCP-originated
        // transitions all land in the In Progress column.
        "in-progress"
    } else if has("ready") {
        "ready"
    } else {
        // unlabeled open issues + "deferred"/"icebox" labels → icebox
        "icebox"
    }
}

/// Shape a single octocrab Issue into the board JSON envelope the
/// Electron renderer expects. Lifted out as a free function so
/// `fetch_board_stories` and `get_story` emit identical shapes — the
/// renderer can reuse the same parser for both endpoints.
fn issue_to_board_json(issue: &octocrab::models::issues::Issue) -> serde_json::Value {
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
    let state = derive_renderer_state(&issue.state, &labels);
    serde_json::json!({
        "number": issue.number,
        "title": issue.title,
        "state": state,
        "labels": labels,
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

    /// #403: full board as typed JSON Values. Same underlying
    /// octocrab call as `list_stories` but returns the richer
    /// issue_to_board_json payload (includes body + state + labels
    /// + assignees + created_at). REST /api/board endpoint +
    ///   list_ready MCP handler both consume this.
    async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
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

    async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        if let Some(ref path) = self.board_fixture {
            let s =
                std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"))?;
            return serde_json::from_str(&s).map_err(|e| format!("parse fixture: {e}"));
        }
        Ok(Vec::new())
    }

    async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
        Ok(Some(serde_json::json!({
            "number": number,
            "title": format!("(fake) story #{number}"),
            "body": "",
            "state": "ready",
            "labels": [],
        })))
    }

    async fn comment_on_story(&self, number: u64, _body: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} commented"))
    }

    async fn list_story_comments(&self, _number: u64) -> Result<Vec<serde_json::Value>, String> {
        Ok(Vec::new())
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

#[cfg(test)]
mod derive_renderer_state_tests {
    use super::*;
    use octocrab::models::IssueState;

    fn labels(ns: &[&str]) -> Vec<String> {
        ns.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn closed_issues_map_to_accepted() {
        assert_eq!(
            derive_renderer_state(&IssueState::Closed, &labels(&["ready"])),
            "accepted"
        );
    }

    #[test]
    fn delivered_wins_over_ready_and_started() {
        assert_eq!(
            derive_renderer_state(
                &IssueState::Open,
                &labels(&["ready", "started", "delivered"])
            ),
            "delivered"
        );
    }

    #[test]
    fn started_or_finished_yield_in_progress() {
        assert_eq!(
            derive_renderer_state(&IssueState::Open, &labels(&["started"])),
            "in-progress"
        );
        assert_eq!(
            derive_renderer_state(&IssueState::Open, &labels(&["finished"])),
            "in-progress"
        );
    }

    // Regression guard: start_story applies LABEL_IN_PROGRESS ("in
    // progress" with a space); the renderer's column grouping keys
    // off the derived "in-progress" state. Mismatch drops the story
    // out of every column (2026-04-22 live incident).
    #[test]
    fn in_progress_label_with_space_yields_in_progress_state() {
        assert_eq!(
            derive_renderer_state(&IssueState::Open, &labels(&["in progress"])),
            "in-progress"
        );
    }

    #[test]
    fn ready_label_yields_ready() {
        assert_eq!(
            derive_renderer_state(&IssueState::Open, &labels(&["Feature", "ready"])),
            "ready"
        );
    }

    #[test]
    fn unlabeled_open_issues_fall_through_to_icebox() {
        assert_eq!(
            derive_renderer_state(&IssueState::Open, &labels(&[])),
            "icebox"
        );
    }
}
