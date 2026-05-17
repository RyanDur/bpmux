//! Workflow signal builders + broadcast markers (#197).
//!
//! Pure functions that compose `ClientMessage` payloads for the
//! story-lifecycle signals fired from the handler layer (deliver →
//! designer/pm; accept → dev; pass_desk_check → ryan/pm; etc.).
//! Plus the structurally-typed broadcast markers
//! (`BOARD_INVALIDATE_MARKER`, `SYNC_STATUS_UPDATE_MARKER`) and the
//! fire-and-log helper that emits a board-invalidate over the WS.
//!
//! Extracted from `bridge::tools_stories` per #1353 to bound module
//! size; the public API is re-exported through `mod.rs::pub use` so
//! existing call-sites don't change.

use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use crate::transport::protocol::ClientMessage;

/// Marker broadcast from every story-mutating MCP handler. Every
/// connected TUI receives it (broadcast — no `to=` filter) and fires
/// a board refetch, so internal-mutation state lands on the board
/// within ~2s instead of waiting up to 60s for the next poll tick.
///
/// Distinct from the existing agent-addressed workflow signals
/// (`#N delivered — desk-check needed`, etc.) which carry semantic
/// payload to specific roles. This marker is system-typed so it
/// doesn't clutter chat and is structurally identifiable without
/// substring matching on signal text.
pub const BOARD_INVALIDATE_MARKER: &str = "__BPMUX_BOARD_INVALIDATE__";

/// Sync-status-change broadcast marker (#1164). Emitted by the
/// bridge's `run_drain_loop` when a drain tick mutates state — i.e.
/// `drain_pending` returned `Ok(n)` with `n > 0` (events flushed) OR
/// `Err(_)` (mutation failed; pending count unchanged but last_error
/// flipped). The renderer's `SyncStatusGlyph` subscribes and refetches
/// `/api/sync-status` immediately, so the user sees state transitions
/// within WS latency instead of waiting for the 5s client-side poll
/// tick. The 5s poll stays as a resilience-fallback for the
/// disconnected-then-reconnected window.
pub const SYNC_STATUS_UPDATE_MARKER: &str = "__BPMUX_SYNC_STATUS_UPDATE__";

/// Build the board-invalidate broadcast signal. System-typed so the
/// TUI recognizes it as an internal marker rather than a chat line.
/// No `to=` set — every agent's TUI receives + refreshes.
pub(crate) fn build_board_invalidate_signal() -> ClientMessage {
    ClientMessage::Send {
        text: BOARD_INVALIDATE_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
    }
}

/// Fire-and-log: emit the board-invalidate broadcast, diag-log on
/// failure. Called by every story-mutating handler alongside their
/// existing semantic signals.
pub(crate) async fn broadcast_board_invalidate(ws_tx: &Mutex<Option<WsSink>>) {
    let msg = build_board_invalidate_signal();
    match serde_json::to_string(&msg) {
        Ok(json) => {
            if let Err(e) = ws_send(ws_tx, json).await {
                crate::session::diag::log_diag(&format!(
                    "bridge: board-invalidate signal failed: {e}"
                ));
            }
        }
        Err(e) => crate::session::diag::log_diag(&format!(
            "bridge: board-invalidate serialize failed: {e}"
        )),
    }
}

pub(crate) fn build_delivery_signals(number: u64, title: &str) -> Vec<ClientMessage> {
    vec![
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered."),
            to: Some(vec!["designer".to_string()]),
            addressee: Some("designer".to_string()),
            message_type: None,
        },
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered."),
            to: Some(vec!["pm".to_string()]),
            addressee: Some("pm".to_string()),
            message_type: None,
        },
    ]
}

pub(crate) fn build_start_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} started."),
        to: Some(vec!["pm".to_string()]),
        addressee: Some("pm".to_string()),
        message_type: None,
    }
}

pub(crate) fn build_accept_signal(number: u64, accepted_by: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} accepted by {accepted_by}."),
        to: None,
        addressee: Some("dev".to_string()),
        message_type: None,
    }
}

pub(crate) fn build_reject_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} rejected: {reason}."),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
    }
}

/// Classify a story's type (Feature/Bug/Chore per #264) and return
/// the addressee who should receive the pass_desk_check signal.
/// Feature → Ryan (owner-gate lane). Bug + Chore → PM (team-accept
/// lane; bugs fixed are internal unless they surface user-visible
/// change, and that's a separate signal decision). Defaults to
/// Feature under the classifier's default-to-Feature rule.
pub(crate) fn classify_for_pass_desk_check(
    number: u64,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> crate::board::domain::StoryType {
    refresh
        .and_then(|r| r.story(number))
        .map(|s| s.story_type)
        .unwrap_or(crate::board::domain::StoryType::Feature)
}

pub(crate) fn addressee_for_pass_desk_check(
    story_type: crate::board::domain::StoryType,
) -> &'static str {
    match story_type {
        crate::board::domain::StoryType::Feature
        | crate::board::domain::StoryType::Epic
        | crate::board::domain::StoryType::Iteration => "ryan",
        crate::board::domain::StoryType::Bug | crate::board::domain::StoryType::Chore => "pm",
    }
}

pub(crate) fn build_pass_desk_check_signal(number: u64, addressee: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} ready for acceptance. passed designer desk-check."),
        to: Some(vec![addressee.to_string()]),
        addressee: Some(addressee.to_string()),
        message_type: None,
    }
}

/// Companion signal to PM on a feature PASS — cues presentation to
/// Ryan. Distinct from the Ryan-facing "ready for acceptance" signal:
/// PM and Ryan have different roles in the accept loop (PM presents,
/// Ryan decides) and the tailored text tells each what to do next.
pub(crate) fn build_pass_desk_check_pm_present_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} passed designer desk-check."),
        to: Some(vec!["pm".to_string()]),
        addressee: Some("pm".to_string()),
        message_type: None,
    }
}

/// Compose the full set of pass_desk_check signals for a story. For
/// features, emits a Ryan-facing "ready for acceptance" signal AND a
/// PM-facing "present to ryan" signal (#259 — previously PM had to
/// scan chat for designer's PASS). For infrastructure, the single
/// PM-facing signal is sufficient since PM is also the acceptor.
pub(crate) fn build_pass_desk_check_signals(
    number: u64,
    story_type: crate::board::domain::StoryType,
) -> Vec<ClientMessage> {
    let primary = addressee_for_pass_desk_check(story_type);
    let mut signals = vec![build_pass_desk_check_signal(number, primary)];
    if primary == "ryan" {
        signals.push(build_pass_desk_check_pm_present_signal(number));
    }
    signals
}

pub(crate) fn build_fail_desk_check_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} failed desk-check: {reason}."),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
    }
}

/// Iteration-close externalize-flush prompt (#282). Fires a broadcast
/// signal with addressee="all" so every agent sees it as their turn
/// to externalize unsaved findings before the iteration closes and
/// mid-chat discoveries risk eviction by compaction. Narrow v1 per
/// the 2026-04-17 team converge — externalize-only, no compound retro
/// or promotion audit. Agents decide what to save through own lens.
pub(crate) fn build_iteration_close_signal() -> ClientMessage {
    ClientMessage::Send {
        text: "[iteration-close] iteration closing — flush unsaved findings now via \
               Write / save_lesson / add_principle / doc edit. pre-close context is \
               still available."
            .to_string(),
        to: None,
        addressee: Some("all".to_string()),
        message_type: None,
    }
}
