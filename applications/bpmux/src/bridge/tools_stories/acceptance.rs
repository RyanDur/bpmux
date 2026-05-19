//! Acceptance gates: accept / reject (terminal-direction), pass /
//! fail desk-check (deliver-direction), plus the bug-question gate
//! that all four consult to block accept-and-file-bug-as-deferral.
//!
//! Each handler fires its workflow signal before the canonical-store
//! mutation so the team learns the intent even when persistence
//! fails, then drains pending events so subsequent mutations see a
//! settled state.

use tokio::sync::Mutex;

use super::super::tools::{WsSink, ws_send};
use super::helpers::{
    LABEL_DELIVERED, LABEL_IN_PROGRESS, LABEL_READY, ensure_story_in_canonical_store,
    reap_workflow_labels, role_from_name,
};
use super::signals::{
    broadcast_board_invalidate, build_accept_signal, build_fail_desk_check_signal,
    build_pass_desk_check_signals, build_reject_signal, classify_for_pass_desk_check,
};

pub(crate) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
    session: Option<&std::sync::Arc<crate::session::Session>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::Accept)?;
    let r = refresh.ok_or(
        "accept_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    r.test_race_window_rendezvous().await;
    let story_opt = r.store().story(number);
    let class = story_opt
        .as_ref()
        .map(|s| crate::workflow::gate_class_from_labels(&s.raw_labels))
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "accept", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    if let Some(s) = session {
        let evidence = recent_evidence_for_story(s, number)?;
        crate::workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
        )
        .map_err(|e| e.as_json_string())?;
    }
    if let Some(story) = story_opt.as_ref() {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
        if let Some(comments) = story.gate_comments.as_deref() {
            let prior_fires: Vec<crate::workflow::PriorFire> = comments
                .iter()
                .map(|gc| crate::workflow::PriorFire {
                    gate_id: gate_to_id(gc.gate).to_string(),
                    actor_role: gc.actor.clone(),
                })
                .collect();
            crate::workflow::check_self_mutation_refused(
                actor_role,
                "accept",
                &prior_fires,
                &["deliver", "pass-desk-check"],
            )
            .map_err(|e| e.as_json_string())?;
        }
    }
    let role = role_from_name(agent_name);
    let signal = serde_json::to_string(&build_accept_signal(number, role))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    accept_via_canonical_store(r, number, actor, bypass).await?;
    r.test_race_window_complete();
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} accepted and closed"))
}

fn gate_to_id(gate: crate::board::domain::Gate) -> &'static str {
    match gate {
        crate::board::domain::Gate::Filed => "filed",
        crate::board::domain::Gate::Pickup => "pickup",
        crate::board::domain::Gate::Delivered => "deliver",
        crate::board::domain::Gate::PassDeskCheck => "pass-desk-check",
        crate::board::domain::Gate::FailDeskCheck => "fail-desk-check",
        crate::board::domain::Gate::Rejected => "reject",
        crate::board::domain::Gate::Accepted => "accept",
    }
}

fn recent_evidence_for_story(
    session: &std::sync::Arc<crate::session::Session>,
    story_number: u64,
) -> Result<Vec<crate::workflow::EvidenceMessage>, String> {
    let store = crate::session::verdicts::load(&session.dir);
    let verdicts = crate::session::verdicts::verdicts_for_story(&store, story_number);
    Ok(verdicts
        .into_iter()
        .map(|v| crate::workflow::EvidenceMessage {
            from_role: v.from_role,
            ts: v.ts,
        })
        .collect())
}

pub(crate) async fn accept_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    actor: String,
    bypass: Option<String>,
) -> Result<(), String> {
    ensure_story_in_canonical_store(refresh, number).await?;
    let meta = crate::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    reap_workflow_labels(refresh.as_ref(), number, &meta, "accepted")?;
    refresh
        .mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Accepted,
            payload: bypass.map(format_bypass_payload),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta: meta.clone(),
            number,
            new_state: crate::board::domain::StoryState::Accepted,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: None,
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{number} drain after accept failed: {e}"))?;
    Ok(())
}

pub(crate) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            crate::workflow::GateError::GateMismatch {
                actor_role: actor_role.to_string(),
                attempted_transition: "reject".to_string(),
                class: crate::workflow::GateClass::None,
                missing_evidence: vec!["reason-cite".to_string()],
            }
            .as_json_string()
        })?
        .to_string();
    let r = refresh.ok_or(
        "reject_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    let story_opt = r.store().story(number);
    let class = story_opt
        .as_ref()
        .map(|s| crate::workflow::gate_class_from_labels(&s.raw_labels))
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "reject", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    if let Some(story) = story_opt.as_ref() {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    let target_state =
        crate::board::domain::StoryState::parse(&transition.to).ok_or_else(|| {
            format!(
                "workflow.toml reject.to '{}' is not a known state",
                transition.to
            )
        })?;
    let signal = serde_json::to_string(&build_reject_signal(number, &reason))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    reject_via_canonical_store(r, number, actor, reason.clone(), target_state).await?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    actor: String,
    reason: String,
    target_state: crate::board::domain::StoryState,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, number).await?;
    let was_accepted = prior.is_closed || prior.state == crate::board::domain::StoryState::Accepted;
    let meta = crate::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    let keep_label = match target_state {
        crate::board::domain::StoryState::Ready => LABEL_READY,
        crate::board::domain::StoryState::InProgress => LABEL_IN_PROGRESS,
        crate::board::domain::StoryState::Delivered => LABEL_DELIVERED,
        _ => LABEL_READY,
    };
    reap_workflow_labels(refresh.as_ref(), number, &meta, keep_label)?;
    if was_accepted {
        refresh
            .mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Rejected,
            payload: Some(reason),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: target_state,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{number} drain after reject failed: {e}"))?;
    Ok(())
}

/// Pass desk-check — designer only. Signals ryan that the story is
/// ready for acceptance (applies the `ready for accept` label). Err
/// is reserved for signal-send failure; gh label edit failures are
/// reported in the Ok status string so callers can observe both
/// outcomes without conflation. Mirrors handle_deliver_story's split
/// (see that doc for rationale).
///
/// Signal recipients depend on story type (Feature/Bug/Chore per
/// #264). Chore + Bug → PM alone (team-accept lane). Feature → Ryan
/// AND PM (#259 — Ryan is the acceptor, PM is the presenter; PM
/// needs a direct signal so presentation doesn't depend on scanning
/// chat for designer's PASS message).
pub(crate) async fn handle_pass_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
    session: Option<&std::sync::Arc<crate::session::Session>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::PassDeskCheck)?;
    let r_opt = refresh;
    let story_opt = r_opt.and_then(|r| r.store().story(number));
    let class = story_opt
        .as_ref()
        .map(|s| crate::workflow::gate_class_from_labels(&s.raw_labels))
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "pass-desk-check", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    if let Some(s) = session {
        let evidence = recent_evidence_for_story(s, number)?;
        crate::workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
        )
        .map_err(|e| e.as_json_string())?;
    }
    let ci_snapshot = refresh.and_then(|r| r.ci_state());
    crate::workflow::check_ci_green(transition, ci_snapshot.as_ref())
        .map_err(|e| e.as_json_string())?;
    let was_closed = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, number).await?;
        if let Some(body) = prior.body.as_deref() {
            crate::bridge::atp_substantive_gate::verify_atp_substantive(body)?;
        }
        prior.is_closed
    } else {
        false
    };
    let story_type = classify_for_pass_desk_check(number, refresh);
    for msg in build_pass_desk_check_signals(number, story_type) {
        let signal = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
        if let Err(e) = ws_send(ws_tx, signal).await {
            crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
        }
    }
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
        reap_workflow_labels(r.as_ref(), number, &meta, LABEL_DELIVERED)?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::PassDeskCheck,
            payload: bypass.clone().map(format_bypass_payload),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::Delivered,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} drain after pass_desk_check failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!(
            "#{number} passed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "pass_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

const FALSIFIER_CITE_MIN_CHARS: usize = 16;

fn label_for_state(state: crate::board::domain::StoryState) -> &'static str {
    use crate::board::domain::StoryState;
    match state {
        StoryState::Icebox => "icebox",
        StoryState::Ready => "ready",
        StoryState::InProgress => "in progress",
        StoryState::DeskCheck => "desk-check",
        StoryState::Delivered => "delivered",
        StoryState::Acceptance => "acceptance",
        StoryState::Accepted => "accepted",
        StoryState::Retired => "retired",
    }
}

pub(crate) async fn handle_fail_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let transition = crate::workflow::resolve_transition(
        workflow,
        "fail-desk-check",
        crate::workflow::GateClass::None,
    )
    .map_err(|e| e.as_json_string())?;
    crate::workflow::check_role_allowed(transition, actor_role).map_err(|e| e.as_json_string())?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let prior = if let Some(r) = refresh {
        Some(ensure_story_in_canonical_store(r, number).await?)
    } else {
        None
    };
    if let Some(story) = prior.as_ref() {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
    }
    let was_accepted = prior
        .as_ref()
        .map(|s| s.is_closed || s.state == crate::board::domain::StoryState::Accepted)
        .unwrap_or(false);
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let trimmed_reason = reason.trim();
    if trimmed_reason.len() < FALSIFIER_CITE_MIN_CHARS {
        let err = serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "fail-desk-check",
            "missing_evidence": ["falsifier-cite"],
            "min_chars": FALSIFIER_CITE_MIN_CHARS,
            "observed_chars": trimmed_reason.len(),
        });
        return Err(err.to_string());
    }
    let target_state =
        crate::board::domain::StoryState::parse(transition.to.as_str()).ok_or_else(|| {
            format!(
                "workflow.toml fail-desk-check transition.to='{}' is not a recognized StoryState",
                transition.to
            )
        })?;
    let signal = serde_json::to_string(&build_fail_desk_check_signal(number, reason))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        if was_accepted {
            r.mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
        }
        let keep_label = label_for_state(target_state);
        reap_workflow_labels(r.as_ref(), number, &meta, keep_label)?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::FailDeskCheck,
            payload: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: target_state,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} drain after fail_desk_check failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!(
            "#{number} failed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "fail_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

#[derive(Clone, Copy)]
pub(crate) enum BugGate {
    Accept,
    PassDeskCheck,
}

impl BugGate {
    fn name(self) -> &'static str {
        match self {
            BugGate::Accept => "accept",
            BugGate::PassDeskCheck => "pass desk-check",
        }
    }

    fn alt_gate(self) -> &'static str {
        match self {
            BugGate::Accept => "reject_story",
            BugGate::PassDeskCheck => "fail_desk_check",
        }
    }

    fn past_tense(self) -> &'static str {
        match self {
            BugGate::Accept => "accepted",
            BugGate::PassDeskCheck => "passed",
        }
    }
}

pub(crate) fn verify_bug_question_or_bypass(
    args: &serde_json::Map<String, serde_json::Value>,
    gate: BugGate,
) -> Result<Option<String>, String> {
    let would_file_bug = args.get("would_file_bug").and_then(|v| v.as_bool()).ok_or_else(|| {
        format!(
            "{} requires 'would_file_bug: bool' argument. Bug-question test: would you file a bug about this if you {} it as-is? If yes -> use {} instead (per `feedback_bug_question_test_at_desk_check`).",
            gate.name(),
            gate.past_tense(),
            gate.alt_gate(),
        )
    })?;
    if !would_file_bug {
        return Ok(None);
    }
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    if !force {
        return Err(format!(
            "Cannot {}: would_file_bug=true. Use {} instead -- accept-and-file-bug-as-deferral is the anti-pattern this gate closes (per `feedback_bug_question_test_at_desk_check`). To override with explicit justification, pass force=true and bypass_reason: \"<reason>\".",
            gate.name(),
            gate.alt_gate(),
        ));
    }
    let reason = args
        .get("bypass_reason")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "{} bypass requires 'bypass_reason: String' (non-empty) -- recorded in gate-comment audit trail per `feedback_external_catch_over_self_catch`.",
                gate.name(),
            )
        })?;
    Ok(Some(reason.to_string()))
}

pub(crate) fn format_bypass_payload(reason: String) -> String {
    format!("[bug-question bypass] {reason}")
}
