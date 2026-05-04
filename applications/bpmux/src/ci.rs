//! Background CI-status polling + indicator state (#151).
//!
//! Phase 1 from `docs/ci-status-ux-spec.md`:
//! - Shell out to `gh run list -L 1 --branch main --json ...` every 60s
//! - Map the result to a three-state enum (Green / Red / Running)
//! - Emit updates over an mpsc channel for the event loop to consume
//! - All polling errors routed through `session::diag::log_diag` so a
//!   network blip or missing `gh` binary never corrupts the TUI frame
//!
//! Explicit non-goals (per the UX spec): no drill-in, no notifications, no
//! broadcast messages, no cross-workspace propagation, no attribution text.
//! The indicator state is *passive* — the only side effect is changing a
//! glyph in the status bar.

use std::collections::HashMap;

use serde::Deserialize;

use crate::session::Message;
use crate::session::message::{DeepDetail, MultiDepthPayload};

/// CI indicator state. `Unknown` is the internal startup state
/// before the first poll — rendered as a dim placeholder.
///
/// `QuotaExhausted` (#397): distinct from `Red` (code-failure) to
/// name the infrastructure-blocked case where the Actions runner
/// never started due to account spending-limit / billing. Team-
/// layer gates (dev pre-deliver / designer pre-PASS / pm pre-accept)
/// accept `quota_exhausted` as pass-with-citation rather than
/// hard-block — owner's lane to resolve; time-boxed by billing
/// reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CiState {
    #[default]
    Unknown,
    Green,
    Red,
    Running,
    QuotaExhausted,
}

/// Richer per-poll CI snapshot — state plus the metadata needed for
/// the broadcast message format. Fields beyond `state` are
/// optional because `gh run list` may return partial data (e.g., a
/// run without a display title, or an older gh version without some
/// fields).
#[derive(Debug, Clone, Default)]
pub struct CiEvent {
    pub state: CiState,
    pub sha: Option<String>,
    pub title: Option<String>,
    pub run_id: Option<u64>,
}

/// Hydrate the poller's `last_broadcast` state from the session log
/// (#300 fix). Scans messages newest→oldest for the most recent CI
/// transition broadcast (Activity with `🔴 CI` / `🟢 CI` prefix) and
/// returns its CiState. If no prior CI event exists, returns None —
/// first-ever poll in a fresh session will then emit honestly.
///
/// Without this, `spawn_poller` initializes `last_broadcast = None`
/// on every spawn; a TUI restart during a Red period re-emits
/// "🔴 CI failed" because compute_broadcast_text sees `None !=
/// Some(Red)` and falls into the emission branch. Hydration closes
/// the gap across the restart boundary so the accepted broadcast
/// contract ("emit only on genuine transitions") holds.
/// Format the broadcast text for a CI poll event. Pure function, no IO
/// — caller (`spawn_poller`) is responsible for tuple-dedup before
/// firing the broadcast.
///
/// #1140 reshape: the prior transition-only signature
/// `(Option<CiState>, &CiEvent) -> Option<String>` dropped
/// GREEN-to-GREEN with new run_id, the canonical "my push went green"
/// case. Dedup responsibility moved to `spawn_poller` (compares
/// `(run_id, state)` tuple); this function decides only how to render
/// terminal-state completions.
///
/// - `Red` → "🔴 CI failed on …"
/// - `Green` → "🟢 CI green on …"
/// - `QuotaExhausted` → "⏸ CI paused (quota exhausted) on …"
/// - `Running` / `Unknown` → silent (None)
///
/// Hydration backward-compat: `hydrate_last_broadcast` matches the
/// "🔴 CI" / "🟢 CI" / "⏸ CI" prefixes; this still satisfies that.
pub(crate) fn compute_broadcast_text(next: &CiEvent) -> Option<String> {
    match next.state {
        CiState::Red => Some(format_failure_line(next)),
        CiState::Green => Some(format_success_line(next)),
        CiState::QuotaExhausted => Some(format_paused_line(next)),
        CiState::Running | CiState::Unknown => None,
    }
}

/// Build the `MultiDepthPayload` for a CI transition broadcast
/// (#188 / #184 child 5). Pure function, no IO — called from
/// `spawn_poller` alongside `compute_broadcast_text` so the payload
/// and the text stay consistent by construction.
///
/// **Rule table** (designed to match #184's progressive disclosure
/// pattern while keeping the schema's per-role default_depth map
/// honest):
/// - `summary` is the shallowest form — `🔴 CI failed` for a failure,
///   `🟢 CI recovered` for a recovery. This is what renders if no
///   role-specific depth applies.
/// - `role_default`:
///   - `pm` → summary. PM's decisions need the binary state, not
///     stage-level detail. Don't crowd the shaping queue view.
///   - `designer` → summary. Designer acts on "is shipping blocked?",
///     not on stage-level debug.
///   - `dev` → detail form (full text including commit sha, title,
///     and run link). Dev's decisions depend on WHY a build failed
///     — stage info + commit context is load-bearing.
/// - `deep`:
///   - When we have a `run_id` from the gh poll, the deep form is
///     an `OnDemand` pointer at `gh run view <run_id>`. Token
///     economy: only the drilling agent pays the cost of fetching
///     the full log.
///   - When we don't have a run_id (degenerate gh output), the
///     deep form is a `Preloaded` placeholder so the payload is
///     still valid. Rare in practice.
///
/// The per-role defaults assume the renderer consumer (#184 child 3)
/// will look up the viewing role's entry in `role_default` and fall
/// back to `summary` if missing. Roles not in the map (e.g., `ryan`
/// itself) fall through to `summary`, which is correct — Ryan's
/// default CI view should be the binary state, same as pm/designer.
pub(crate) fn build_multi_depth_for_transition(event: &CiEvent) -> MultiDepthPayload {
    let (summary, dev_detail) = match event.state {
        CiState::Green => ("\u{1F7E2} CI green".to_string(), format_success_line(event)),
        CiState::Red => (
            "\u{1F534} CI failed".to_string(),
            format_failure_line(event),
        ),
        CiState::QuotaExhausted => ("\u{23F8} CI paused".to_string(), format_paused_line(event)),
        CiState::Running | CiState::Unknown => (
            "CI \u{2026}".to_string(),
            "CI in progress or unknown".to_string(),
        ),
    };

    let mut role_default: HashMap<String, String> = HashMap::new();
    role_default.insert("pm".to_string(), summary.clone());
    role_default.insert("designer".to_string(), summary.clone());
    role_default.insert("dev".to_string(), dev_detail);

    let deep = match event.run_id {
        Some(id) => DeepDetail::OnDemand {
            command: format!("gh run view {id}"),
            description: "full CI run output including all stages and logs".to_string(),
        },
        None => DeepDetail::Preloaded {
            text: "(no run id available)".to_string(),
        },
    };

    MultiDepthPayload {
        summary,
        role_default,
        deep,
    }
}

/// GitHub Actions URL for this repo. Hardcoded — bpmux lives in one
/// repo; configurability is speculative.
pub fn github_actions_url() -> &'static str {
    "https://github.com/RyanDur/bpmux/actions"
}

/// Build the dev-addressed workflow signal for a CI transition (#215).
/// Pure helper: takes the broadcast text computed by
/// `compute_broadcast_text` and wraps it as a Chat message addressed
/// to dev so the bridge pushes it into dev's notifications. Same
/// addressing pattern as `deliver_story → designer` signals.
pub(crate) fn build_dev_ci_signal(text: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: text.to_string(),
        ts: chrono::Utc::now(),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        ..Message::default()
    }
}

fn format_failure_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F534} CI failed");
    append_meta(&mut line, event);
    line
}

fn format_success_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F7E2} CI green");
    append_meta(&mut line, event);
    line
}

fn format_paused_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{23F8} CI paused (quota exhausted)");
    append_meta(&mut line, event);
    line
}

/// Append the optional metadata tail (short SHA + title + run link)
/// to a message line. Each field is conditional so a partial poll
/// still produces a useful message.
fn append_meta(line: &mut String, event: &CiEvent) {
    if let Some(sha) = event.sha.as_deref() {
        let short: String = sha.chars().take(7).collect();
        line.push_str(" \u{2014} ");
        line.push_str(&short);
        if let Some(title) = event.title.as_deref() {
            line.push_str(" \"");
            // Truncate long titles so the one-line format stays scannable.
            const MAX_TITLE_LEN: usize = 60;
            if title.chars().count() > MAX_TITLE_LEN {
                let truncated: String = title.chars().take(MAX_TITLE_LEN - 1).collect();
                line.push_str(&truncated);
                line.push('\u{2026}');
            } else {
                line.push_str(title);
            }
            line.push('"');
        }
    }
    if let Some(run_id) = event.run_id {
        line.push_str(" \u{2192} gh run view ");
        line.push_str(&run_id.to_string());
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct RunEntry {
    pub status: String,
    pub conclusion: Option<String>,
    // REST API uses `head_sha` / `display_title` / `id`; retain aliases
    // for the legacy `gh --json headSha,displayTitle,databaseId` format
    // so any persisted-JSON or fixture data from before #320 still parses.
    #[serde(alias = "headSha", default)]
    pub head_sha: Option<String>,
    #[serde(alias = "displayTitle", default)]
    pub display_title: Option<String>,
    #[serde(rename = "id", alias = "databaseId", default)]
    pub database_id: Option<u64>,
    /// #397: annotation message from the check-run, surfaced when
    /// conclusion=failure so `classify_runs` can distinguish
    /// infrastructure-blocked runs (quota-exhausted / billing) from
    /// code-failure runs. Populated by the tracker's annotation
    /// fetch on failed runs; `None` for successful runs (annotation
    /// fetch skipped — no distinction needed).
    #[serde(default, skip)]
    pub annotation_message: Option<String>,
}

/// Pattern recognizing GitHub Actions' quota / billing exhaustion
/// annotation. Text comes verbatim from the Actions API on a failed-
/// to-start run when the owner's spending limit is reached:
///   "The job was not started because recent account payments have
///    failed or your spending limit needs to be increased..."
/// Match on the canonical prefix "job was not started because" —
/// GitHub uses this for any runner-unavailable reason (billing,
/// disabled workflow, insufficient minutes). All of those map to
/// `QuotaExhausted` (team's gate semantics: infra-unavailable,
/// owner's lane to resolve).
pub(crate) fn annotation_indicates_quota_exhaustion(msg: &str) -> bool {
    msg.contains("job was not started because")
        || msg.contains("spending limit")
        || msg.contains("billing")
}

pub fn classify_runs(runs: &[RunEntry]) -> CiEvent {
    let Some(latest) = runs.first() else {
        return CiEvent::default();
    };
    // "Running wins over stale success or stale failure — the user cares
    // that a fresh answer is coming." (spec)
    let state = if latest.status == "in_progress" || latest.status == "queued" {
        CiState::Running
    } else {
        match latest.conclusion.as_deref() {
            Some("success") => CiState::Green,
            // #397: distinguish quota-exhausted / infrastructure-
            // blocked failures from genuine code failures. The
            // annotation message (populated upstream by the tracker
            // for failed runs) carries GitHub's reason; if it matches
            // the quota-exhaustion pattern, map to QuotaExhausted
            // (team-gate pass-with-citation). Otherwise map to Red
            // (genuine failure — block gates).
            Some(_) => match latest.annotation_message.as_deref() {
                Some(msg) if annotation_indicates_quota_exhaustion(msg) => CiState::QuotaExhausted,
                _ => CiState::Red,
            },
            None => CiState::Unknown,
        }
    };
    CiEvent {
        state,
        sha: latest.head_sha.clone(),
        title: latest.display_title.clone(),
        run_id: latest.database_id,
    }
}
