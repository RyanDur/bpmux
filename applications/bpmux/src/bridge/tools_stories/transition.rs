//! Dev-lane state-transition handlers: `start_story` (Ready →
//! InProgress) and `deliver_story` (InProgress → DeskCheck). Both
//! require non-empty paraphrase / verified-observation fields per
//! the 2026-04-22 + 2026-04-23 retros' externalization model, and
//! both `drain_pending` before returning so subsequent mutations see
//! a fully-settled state (sister to the close/accept handlers).

use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use super::helpers::{
    LABEL_IN_PROGRESS, ensure_story_in_canonical_store, reap_workflow_labels,
    require_nonempty_string, require_role, validate_acceptance_test_path_at_deliver,
    validate_acceptance_test_path_at_pickup,
};
use super::signals::{broadcast_board_invalidate, build_delivery_signals, build_start_signal};

/// Signal fires on INTENT (before gh) so the team is notified even
/// when the gh label edit fails.
///
/// #398 pickup-gate: dev must articulate a pickup paraphrase +
/// falsifier reframing BEFORE the in-progress label fires. Missing
/// or empty values fail fast before any signal or label mutation so
/// the board doesn't transition to in-progress on a bypassed gate.
/// The paraphrase + falsifier reframing land as a Gate::Pickup
/// gate-comment for audit-trail parity with deliver_story's
/// verified_observation. drain_pending before return avoids the
/// #1292 race between in-progress label tagging and a subsequent
/// close_story.
pub(crate) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "start")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let was_closed = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, number).await?;
        let _ = r.fetch_body(number).await;
        prior.is_closed
    } else {
        false
    };
    let pickup_paraphrase = require_nonempty_string(
        args,
        "pickup_paraphrase",
        "pickup_paraphrase is required (2026-04-23 retro): restate the user-job as you understand it at pickup time. This broadcast is what separates 'dev is working' from 'dev is silent' to the rest of the team.",
    )?;
    let falsifier_reframing = require_nonempty_string(
        args,
        "falsifier_reframing",
        "falsifier_reframing is required (2026-04-23 retro): restate the observation-falsifier — either verbatim from filing or your sharpening. Forces engagement with the rejection criteria before code.",
    )?;
    if let Some(r) = refresh {
        validate_acceptance_test_path_at_pickup(args, number, r.as_ref())?;
    }
    let signal =
        serde_json::to_string(&build_start_signal(number)).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let pickup_payload =
        format!("{pickup_paraphrase}\n\n---\n\n**falsifier_reframing**:\n\n{falsifier_reframing}");
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
        }
        reap_workflow_labels(r.as_ref(), number, &meta, LABEL_IN_PROGRESS)?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Pickup,
            payload: Some(pickup_payload.clone()),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::InProgress,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} start push failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} marked as in progress"));
    }
    Err(
        "start_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// Deliver a story — dev only.
///
/// Err is reserved for SIGNAL failures — the load-bearing path that
/// notifies the team a story is ready for desk-check. gh label edits
/// are a best-effort side effect; their failure is reported in the
/// Ok status string so callers can observe both outcomes without
/// conflating them.
///
/// #391 deliver-gate: dev articulates what they drove end-to-end
/// before delivery. The verified_observation becomes the diff-point
/// designer checks at desk-check (claim-vs-reality) and persists as
/// a Gate::Delivered gate-comment.
pub(crate) async fn handle_deliver_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "deliver")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let refreshed: Option<Option<crate::board::domain::Story>> = match refresh {
        Some(r) => {
            let story = ensure_story_in_canonical_store(r, number).await?;
            let _ = r.fetch_body(number).await;
            Some(Some(story))
        }
        None => None,
    };
    if let Some(r) = refresh {
        validate_acceptance_test_path_at_deliver(number, r.as_ref())?;
        if let Some(story) = refreshed.as_ref().and_then(|s| s.as_ref())
            && let Some(body) = story.body.as_deref()
        {
            crate::bridge::deliver_story_body_diff_gate::verify_deliver_drift(
                body,
                story,
                &|args| crate::bridge::tools::git_repo::run_git(args),
            )?;
        }
    }
    let verified_observation = require_nonempty_string(
        args,
        "verified_observation",
        "verified_observation is required (2026-04-22 retro): articulate what you DROVE end-to-end in the running app, not 'tests pass'. Describe the falsifier scenario you exercised + what you observed. Becomes the diff-point at desk-check.",
    )?;
    let title = refreshed
        .as_ref()
        .and_then(|s| s.as_ref())
        .map(|story| story.title.clone())
        .unwrap_or_else(|| format!("#{number}"));
    for signal in build_delivery_signals(number, &title) {
        let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
        if let Err(e) = ws_send(ws_tx, json).await {
            crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
        }
    }
    if let Some(r) = refresh {
        let was_closed = refreshed
            .as_ref()
            .and_then(|s| s.as_ref())
            .map(|s| s.is_closed)
            .unwrap_or(false);
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
        }
        reap_workflow_labels(r.as_ref(), number, &meta, "desk-check")?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Delivered,
            payload: Some(verified_observation.to_string()),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::DeskCheck,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} drain after deliver failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} delivered: signals sent, labels updated"));
    }
    Err(
        "deliver_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
