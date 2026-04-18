use std::sync::OnceLock;

use super::tools_stories::gh;

/// Install rustls's ring crypto provider once per process.
/// rustls-0.23 panics on first TLS handshake without an explicit
/// provider when multiple backends are compiled in.
pub(super) fn ensure_crypto_provider_installed() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

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
}

#[derive(Clone)]
pub struct GhTracker {
    pub board_fixture: Option<String>,
}

impl StoryTracker for GhTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        gh(&["issue", "close", &number.to_string()]).await
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        gh(&["issue", "edit", &number.to_string(), "--add-label", label]).await
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        gh(&[
            "issue",
            "edit",
            &number.to_string(),
            "--remove-label",
            label,
        ])
        .await
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<String, String> {
        let mut args = vec!["issue", "create", "--title", title];
        if !body.is_empty() {
            args.extend(["--body", body]);
        }
        gh(&args).await
    }

    async fn list_stories(&self) -> Result<String, String> {
        if let Some(ref path) = self.board_fixture {
            return std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"));
        }
        gh(&[
            "issue",
            "list",
            "--state",
            "all",
            "--limit",
            "200",
            "--json",
            "number,title,state,labels",
        ])
        .await
    }

    async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        gh(&["issue", "comment", &number.to_string(), "--body", body]).await
    }

    async fn get_story_title(&self, number: u64) -> Option<String> {
        gh(&["issue", "view", &number.to_string(), "--json", "title"])
            .await
            .ok()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|v| v["title"].as_str().map(String::from))
    }

    async fn get_story_labels(&self, number: u64) -> Vec<String> {
        match gh(&["issue", "view", &number.to_string(), "--json", "labels"]).await {
            Ok(json) => match serde_json::from_str::<serde_json::Value>(&json) {
                Ok(v) => v["labels"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|l| l["name"].as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                Err(e) => log_labels_fetch_failure("gh parse", number, e),
            },
            Err(e) => log_labels_fetch_failure("gh", number, e),
        }
    }
}

/// Octocrab-backed StoryTracker. Issues HTTP requests directly
/// to GitHub's REST API instead of shelling out to `gh`. Uses the
/// same octocrab dependency introduced in #219 for the board poller.
///
/// Constructed from an authenticated `Octocrab` client + owner/repo
/// strings. Production path builds it from `GH_TOKEN`/`GITHUB_TOKEN`
/// + `BPMUX_REPO` env vars; see `OctocrabStoryTracker::from_env`.
///
/// Keeps `GhTracker` available as fallback for environments where
/// direct API access isn't configured (no token, no repo env var).
#[derive(Clone)]
pub struct OctocrabStoryTracker {
    client: octocrab::Octocrab,
    owner: String,
    repo: String,
}

/// Result of inspecting env vars to pick a tracker backend. Separates
/// the env-decisioning (pure, testable) from the octocrab client
/// build (impure, requires network config). Only `Octocrab` is a
/// successful selection; the `Fallback` variant names WHY we're
/// falling back to `GhTracker` so the bridge can surface the reason
/// in diag logs instead of silently swapping backends (#298).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TrackerSelection {
    Octocrab {
        owner: String,
        repo: String,
        token: String,
    },
    Fallback(FallbackReason),
}

/// Named reasons for falling back from `OctocrabStoryTracker` to
/// `GhTracker`. Naming them as an enum (instead of a None return)
/// makes the dispatch decision observable — diag logs can cite the
/// specific reason, and unit tests can assert exact variants rather
/// than an opaque "it fell back."
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
    /// config is missing — caller falls back to GhTracker in that
    /// case so the bridge stays functional without API access.
    ///
    /// Logs the fallback reason to diag on None — previously this
    /// was a silent swap, invisible to developers debugging why
    /// their real-gh calls went through the gh CLI instead of
    /// octocrab. Named reasons (NoRepo / MalformedRepo / NoToken)
    /// make the dispatch decision auditable (#298).
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
                    "OctocrabStoryTracker::from_env: falling back to GhTracker ({reason:?})"
                ));
                None
            }
        }
    }

    fn classify(err: octocrab::Error) -> String {
        // Shared GitHub-error taxonomy lives in `crate::github` so
        // the bridge and the board poller both depend on it instead
        // of one reaching across into the other's layer.
        crate::github::classify_error(&err).user_message()
    }
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
}
