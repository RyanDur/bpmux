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
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::mpsc;

use crate::session::diag::log_diag;
use crate::session::message::{DeepDetail, MultiDepthPayload};
use crate::session::{Message, MessageType, Session};

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

const POLL_INTERVAL: Duration = Duration::from_secs(60);

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
pub(super) fn hydrate_last_broadcast(messages: &[Message]) -> Option<CiState> {
    messages.iter().rev().find_map(|m| {
        if m.message_type != MessageType::Activity {
            return None;
        }
        if m.text.starts_with("\u{1F534} CI") {
            Some(CiState::Red)
        } else if m.text.starts_with("\u{1F7E2} CI") {
            Some(CiState::Green)
        } else if m.text.starts_with("\u{23F8} CI") {
            Some(CiState::QuotaExhausted)
        } else {
            None
        }
    })
}

/// Spawn a background task that polls CI every 60 seconds and sends state
/// updates to `tx`. The task exits cleanly when the receiver is dropped.
///
/// Only sends an update when the state CHANGES. The startup transition
/// from `Unknown` → first known state always fires.
///
/// When `broadcast_session` is `Some`, this function also posts an
/// activity-type broadcast message (#106) on meaningful state
/// transitions: failures always surface, recoveries surface only
/// when they follow a prior failure. The transition logic is in
/// `compute_broadcast_text`, which is pure and unit-tested.
///
/// `initial_last_broadcast` is the hydrated value for `last_broadcast`,
/// typically computed via `hydrate_last_broadcast(&app.msg.messages)`
/// at call-site (#300). Pass `None` only for fresh-session cases; on
/// restart during a known CI state, pass the hydrated value so the
/// first poll doesn't re-emit a phantom transition.
pub(super) fn spawn_poller(
    tx: mpsc::Sender<CiState>,
    broadcast_session: Option<Arc<Session>>,
    initial_last_broadcast: Option<CiState>,
) {
    tokio::spawn(async move {
        let mut last_state = CiState::Unknown;
        let mut last_broadcast: Option<CiState> = initial_last_broadcast;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        // First tick fires immediately, so we get a state update at startup
        // rather than waiting a full minute for the first paint.
        loop {
            interval.tick().await;
            let event = match poll_once().await {
                Ok(ev) => ev,
                Err(e) => {
                    log_diag(&format!("ci: poll error: {e}"));
                    // Keep the last known state on error — spec says Phase 1
                    // does not render an error glyph. The diag log captures
                    // the failure for later inspection.
                    continue;
                }
            };
            if event.state != last_state {
                last_state = event.state;
                if tx.send(event.state).await.is_err() {
                    break;
                }
            }
            // Broadcast hook (#106 / #188). Pure transition logic in
            // `compute_broadcast_text`; this branch is just IO. The
            // multi-depth payload (#188) comes from the paired pure
            // helper `build_multi_depth_for_transition`, which
            // populates the per-role default depths and the OnDemand
            // deep-detail pointer at `gh run view <run_id>`.
            if let Some(session) = broadcast_session.as_ref()
                && let Some(text) = compute_broadcast_text(last_broadcast, &event)
            {
                let is_recovery = event.state == CiState::Green;
                let multi_depth = build_multi_depth_for_transition(&event, is_recovery);
                let msg = Message {
                    from: "bpmux".to_string(),
                    text: text.clone(),
                    ts: chrono::Utc::now(),
                    message_type: MessageType::Activity,
                    multi_depth: Some(multi_depth),
                    ..Message::default()
                };
                if let Err(e) = session.broadcast(&msg) {
                    log_diag(&format!("ci: broadcast error: {e}"));
                }
                // Also fire a dev-addressed workflow signal so
                // the dev agent sees CI transitions in their chat
                // notifications without Ryan having to relay. Same
                // pattern as deliver_story → designer signals.
                let dev_signal = build_dev_ci_signal(&text);
                if let Err(e) = session.broadcast(&dev_signal) {
                    log_diag(&format!("ci: dev signal error: {e}"));
                }
                last_broadcast = Some(event.state);
            }
        }
    });
}

/// Decide whether a state change warrants a broadcast message, and
/// return the formatted text if so. Pure function, no IO — tested
/// exhaustively over the transition table below.
///
/// Rules (per PM shape for #106):
/// - Emit "CI failed" whenever `next.state` is Red AND
///   `last_broadcast` was not Red. Covers unknown→red, green→red,
///   running→red. The first known state being red fires on startup.
/// - Emit "CI recovered" whenever `next.state` is Green AND
///   `last_broadcast` was Red. The "recoveries after failures only"
///   rule — a healthy pipeline never broadcasts green.
/// - Everything else: silent. Running/Unknown are ignored.
pub(super) fn compute_broadcast_text(
    last_broadcast: Option<CiState>,
    next: &CiEvent,
) -> Option<String> {
    match next.state {
        CiState::Red if last_broadcast != Some(CiState::Red) => Some(format_failure_line(next)),
        CiState::Green
            if matches!(
                last_broadcast,
                Some(CiState::Red) | Some(CiState::QuotaExhausted)
            ) =>
        {
            Some(format_recovery_line(next))
        }
        CiState::QuotaExhausted if last_broadcast != Some(CiState::QuotaExhausted) => {
            Some(format_paused_line(next))
        }
        _ => None,
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
pub(super) fn build_multi_depth_for_transition(
    event: &CiEvent,
    is_recovery: bool,
) -> MultiDepthPayload {
    let summary = if is_recovery {
        "\u{1F7E2} CI recovered".to_string()
    } else {
        "\u{1F534} CI failed".to_string()
    };
    let dev_detail = if is_recovery {
        format_recovery_line(event)
    } else {
        format_failure_line(event)
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

/// Open a URL in the user's default browser. Fire-and-forget: failure
/// is logged to diag (the user sees nothing happen; better than
/// panicking the TUI). Platform dispatch uses the conventional
/// command for each OS.
pub fn open_in_browser(url: &str) {
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(target_os = "linux")]
    let program = "xdg-open";
    #[cfg(target_os = "windows")]
    let program = "start";
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        log_diag(&format!("open_in_browser: unsupported platform; url={url}"));
        return;
    }
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    if let Err(e) = std::process::Command::new(program).arg(url).spawn() {
        log_diag(&format!("open_in_browser: failed to spawn {program}: {e}"));
    }
}

/// Build the dev-addressed workflow signal for a CI transition (#215).
/// Pure helper: takes the broadcast text computed by
/// `compute_broadcast_text` and wraps it as a Chat message addressed
/// to dev so the bridge pushes it into dev's notifications. Same
/// addressing pattern as `deliver_story → designer` signals.
pub(super) fn build_dev_ci_signal(text: &str) -> Message {
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

fn format_recovery_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F7E2} CI recovered");
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

async fn poll_once() -> Result<CiEvent, Box<dyn std::error::Error + Send + Sync>> {
    // #320: was `gh run list --branch main -L 1 --json ...`. Now calls the
    // Actions REST API directly via octocrab so bpmux has no runtime
    // dependency on the gh CLI. Env missing → error bubbles up and
    // `spawn_poller`'s existing error handling absorbs it (diag log,
    // keep-last-state).
    let (client, owner, repo) = crate::github::octocrab_client()?;
    #[derive(serde::Deserialize)]
    struct RunsResponse {
        workflow_runs: Vec<RunEntry>,
    }
    let path = format!("/repos/{owner}/{repo}/actions/runs?branch=main&per_page=1");
    let response: RunsResponse = client.get(path, None::<&()>).await?;
    Ok(classify_runs(&response.workflow_runs))
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

/// Return the (glyph, color-hint) pair for the status bar. Unknown
/// renders a dim `CI ?` placeholder so the affordance is visible at
/// TUI boot — silent absence during the first-poll window reads as
/// "indicator broken," which it isn't.
pub(super) fn render_indicator(state: CiState) -> (&'static str, CiGlyphColor) {
    match state {
        CiState::Green => ("CI \u{2713}", CiGlyphColor::Green), // ✓
        CiState::Red => ("CI \u{2717}", CiGlyphColor::Red),     // ✗
        CiState::Running => ("CI \u{25CB}", CiGlyphColor::Yellow), // ◌
        // #397 quota-exhausted: pause-glyph ⏸ + amber (Yellow).
        // Distinct shape from the in-progress ring so readers see
        // "paused for infrastructure" vs "running, waiting."
        CiState::QuotaExhausted => ("CI \u{23F8}", CiGlyphColor::Yellow), // ⏸
        CiState::Unknown => ("CI ?", CiGlyphColor::Dim),
    }
}

/// Color-hint returned by `render_indicator`. The caller maps this to its
/// own theme-specific color; keeping it as an enum avoids pulling the
/// theme module into `ci.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CiGlyphColor {
    Green,
    Red,
    Yellow,
    /// Dim / de-emphasized — used for the Unknown placeholder so the
    /// glyph is visible-but-quiet while waiting for the first poll.
    Dim,
}
