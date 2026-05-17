//! Story management tool handlers.
//!
//! Lane-aware operations against the RefreshDriver-backed BoardStore
//! (#451 retired the standalone tracker abstraction). Workflow
//! signals fire on INTENT (before the source-of-truth mutation) so
//! the team is notified even when the mutation fails.

use tokio::sync::Mutex;

use super::tools::{WsSink, ws_send};
use crate::board::label_codec::LabelCodec;

mod helpers;
mod signals;
mod view;

pub use helpers::parse_acceptance_test_path;
#[allow(unused_imports)]
pub(crate) use helpers::{
    ALL_STATE_LABELS, KEEP_NONE, LABEL_DELIVERED, LABEL_IN_PROGRESS, LABEL_READY,
    LABEL_READY_FOR_ACCEPT, compose_iteration_body, compose_story_body,
    ensure_story_in_canonical_store, find_repo_root, reap_workflow_labels, require_nonempty_string,
    require_role, resolve_acceptance_test_path, role_from_name, stale_workflow_labels,
    validate_acceptance_test_path_at_deliver, validate_acceptance_test_path_at_pickup,
};
pub use signals::{BOARD_INVALIDATE_MARKER, SYNC_STATUS_UPDATE_MARKER};
#[allow(unused_imports)]
pub(crate) use signals::{
    addressee_for_pass_desk_check, broadcast_board_invalidate, build_accept_signal,
    build_board_invalidate_signal, build_delivery_signals, build_fail_desk_check_signal,
    build_iteration_close_signal, build_pass_desk_check_pm_present_signal,
    build_pass_desk_check_signal, build_pass_desk_check_signals, build_reject_signal,
    build_start_signal, classify_for_pass_desk_check,
};
pub(crate) use view::handle_view_board;

pub(super) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    if role_from_name(agent_name) != "pm" {
        return Err(
            "create_story is pm-lane. If you have a pain to file, surface it in broadcast chat \
             and pm will file the story."
                .to_string(),
        );
    }
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    // #391: user_job + observation_falsifier are required per the
    // 2026-04-22 retro's 4-gate externalization model. Validation
    // errors name the missing field AND surface a retro-framing
    // pointer so the caller understands WHY the field is required
    // (discoverable-error requirement per pickup-gate falsifier).
    let user_job = require_nonempty_string(
        args,
        "user_job",
        "user_job is required (2026-04-22 retro): 'When <situation>, <role> needs <affordance> because <constraint/goal>.' Cannot file a story without articulating the user-job it answers.",
    )?;
    let observation_falsifier = require_nonempty_string(
        args,
        "observation_falsifier",
        "observation_falsifier is required (2026-04-22 retro): 'I would reject this story if I observed X.' Frames acceptance as observation-that-fails rather than mechanism-that-passes.",
    )?;
    let acceptance_test_path = args
        .get("acceptance_test_path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let author_body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let composed_body = compose_story_body(
        user_job,
        observation_falsifier,
        acceptance_test_path,
        author_body,
    );
    let parent_epic = args.get("parent_epic").and_then(|v| v.as_u64());
    if let Some(r) = refresh {
        if let Some(epic_number) = parent_epic {
            // #1194: read parent from canonical store (ingest poller
            // keeps it fresh ≤60s); refresh_story removed per
            // "MCP actions only pull from / mutate the store".
            let parent = r.story(epic_number).ok_or_else(|| {
                format!(
                    "epic #{epic_number} not found in canonical store — wait for next ingest tick or call refresh_story explicitly"
                )
            })?;
            if parent.story_type != crate::board::domain::StoryType::Epic {
                return Err(format!(
                    "story #{epic_number} is not an Epic (story_type={:?}); only Epic stories can be parents",
                    parent.story_type
                ));
            }
        }
        let number = r
            .create_story(title.to_string(), composed_body.clone())
            .await
            .map_err(|e| format!("create_story failed: {e}"))?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        let story_type = crate::board::domain::story_type_from_labels_or_title(&[], title);
        r.mutate_sync(crate::board::BoardEvent::StoryCreated {
            meta: meta.clone(),
            number,
            title: title.to_string(),
            body: composed_body,
            initial_state: crate::board::domain::StoryState::Icebox,
            story_type,
        })
        .map_err(|e| format!("#{number} created but enqueue failed: {e}"))?;
        if let Some(epic_number) = parent_epic {
            r.mutate_sync(crate::board::BoardEvent::ParentEpicSet {
                meta,
                number,
                epic_number,
            })
            .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
        }
        broadcast_board_invalidate(ws_tx).await;
        return match parent_epic {
            Some(epic_number) => Ok(format!(
                "story created: #{number} {title} (under epic #{epic_number})"
            )),
            None => Ok(format!("story created: #{number} {title}")),
        };
    }
    Err(
        "create_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// #1297 Epic #1189 architectural-pivot slice 1: create a GH issue
/// that represents an iteration itself (sister to Epic story_type).
/// Replaces local markers-store substrate for iteration-marker
/// representation — see ops doc + Epic #1189 architectural-pivot
/// chain (#1297-#1302). The iteration-issue's title is
/// "Iteration {theme_name}" + carries `iteration` label so
/// `story_type_from_labels_or_title` decodes it as
/// `StoryType::Iteration`. Duplicate theme_name (existing OPEN
/// iteration with same name) is rejected at create-time with a
/// named-collision error — sister-discipline to #1281.
pub(super) async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    if role_from_name(agent_name) != "pm" {
        return Err(
            "create_iteration is pm-lane. Iteration entities are PM-owned per Epic #1189."
                .to_string(),
        );
    }
    let theme_name = require_nonempty_string(
        args,
        "theme_name",
        "theme_name is required: short name for the iteration (e.g., 'Aquila', 'Pegasus'). Surfaces as the GH issue title 'Iteration {theme_name}'.",
    )?;
    let r = refresh
        .ok_or("create_iteration requires refresh (RefreshDriver) — canonical store unavailable")?;
    let existing_open_iteration = r.all_stories().into_iter().find(|s| {
        s.story_type == crate::board::domain::StoryType::Iteration
            && !s.is_closed
            && s.title
                .eq_ignore_ascii_case(&format!("Iteration {theme_name}"))
    });
    if let Some(existing) = existing_open_iteration {
        return Err(format!(
            "iteration named '{theme_name}' already open as #{n}; close it first or pick a different name (#1281 sister-discipline: duplicate-collision rejected at create-time)",
            n = existing.number
        ));
    }
    let title = format!("Iteration {theme_name}");
    let body = compose_iteration_body(theme_name);
    let number = r
        .create_story(title.clone(), body.clone())
        .await
        .map_err(|e| format!("create_iteration adapter call failed: {e}"))?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.clone(),
        body: body.clone(),
        initial_state: crate::board::domain::StoryState::Icebox,
        story_type: crate::board::domain::StoryType::Iteration,
    })
    .map_err(|e| format!("#{number} iteration created but enqueue failed: {e}"))?;
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta,
        number,
        label: "iteration".to_string(),
    })
    .map_err(|e| format!("#{number} iteration label enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("iteration created: #{number} {title}"))
}

/// Signal fires on INTENT (before gh) so the team is notified
/// even when the gh label edit fails.
///
/// #398 pickup-gate: dev must articulate a pickup paraphrase +
/// falsifier reframing BEFORE the in-progress label fires. Missing
/// or empty values fail fast before any signal or label mutation
/// so the board doesn't transition to in-progress on a bypassed
/// gate. The paraphrase lands as an issue comment for audit-trail
/// parity with deliver_story's verified_observation.
pub(super) async fn handle_start_story(
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
    // #398: persist the pickup paraphrase + falsifier reframing as
    // an issue comment so designer + pm see dev's interpretation at
    // pickup time (not just at delivery). Mirrors the deliver-gate
    // verified_observation comment pattern (#391).
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
        // #518 slice 1: universal-reap via shared helper. Mirrors the
        // forward-direction pattern from #533 + symmetric to #519/#503
        // terminal-direction reaps. Closes the rework-pickup-from-
        // Rejected state-machine-stuck class observed during #514.
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
        // #1292: drain the outbox before returning so callers see a
        // fully-settled state. Without this, start_story enqueued
        // labels (icebox → in-progress) ride on the background
        // drain_loop tick (1s interval); subsequent close_story would
        // race against still-in-flight tag-mutations. Under Linux-CI
        // workers=4 + repeat-each=3, the 1.5s test-side wait wasn't
        // sufficient — flake-class. Sister-shape to deliver_story /
        // close_story / accept_story which already drain before
        // returning. The asymmetry (start_story alone didn't drain)
        // was the race-source.
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
/// conflating them. Before this split (pre-#199 fix), a gh failure
/// returned Err and test callers used `let _ =` to tolerate it —
/// which also silently masked real signal failures.
pub(super) async fn handle_deliver_story(
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
    // #391 deliver-gate: dev must articulate what they drove end-to-
    // end before delivery. Becomes the diff-point designer checks at
    // desk-check (claim-vs-reality). Missing/empty → validation
    // failure + error message names the field + the retro framing.
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
    // #391: persist the verified_observation as a comment on the
    // story so designer + pm see it at desk-check + accept gates.
    // Comment is auditable; fits the existing comment substrate.
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
        // #518 slice 1: universal-reap via shared helper.
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

// #392: accepts an `operation` arg of "add" (default) or "remove".
// The remove path closes the earlier bypass — before this story,
// label removal required raw `gh issue edit --remove-label`, which
// mutated GitHub without firing the bridge's BOARD_INVALIDATE
// broadcast and stranded peer boards on stale state. Both paths
// now broadcast on success; errors bubble up without broadcasting
// so peers aren't asked to refresh against a no-op. Remove is
// idempotent — the tracker maps GitHub's 404 ("label not attached")
// to Ok so callers can safely remove-without-checking.
pub(super) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "label")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .ok_or("missing 'label' argument")?;
    let operation = args
        .get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("add");
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        let (event, ok_msg) = match operation {
            "add" => (
                crate::board::BoardEvent::TagAdded {
                    meta,
                    number,
                    label: label.to_string(),
                },
                format!("#{number} labeled '{label}'"),
            ),
            "remove" => (
                crate::board::BoardEvent::TagRemoved {
                    meta,
                    number,
                    label: label.to_string(),
                },
                format!("#{number} unlabeled '{label}'"),
            ),
            other => {
                return Err(format!(
                    "invalid 'operation' argument '{other}': must be 'add' or 'remove'"
                ));
            }
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{number} enqueue failed: {e}"))?;
        // #1151: drain dispatches LabelsRefreshed from the
        // tracker's post-mutation labels list, so the local store
        // reflects post-label state without a follow-up
        // refresh_story round-trip.
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} label push failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(ok_msg);
    }
    Err(
        "label_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(super) async fn handle_set_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "set parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let epic_number = args
        .get("epic_number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'epic_number' argument")?;
    if number == epic_number {
        return Err(format!("story #{number} cannot be its own parent epic"));
    }
    let r = refresh.ok_or("set_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let parent = ensure_story_in_canonical_store(r, epic_number).await?;
    if parent.story_type != crate::board::domain::StoryType::Epic {
        return Err(format!(
            "story #{epic_number} is not an Epic (story_type={:?}); only Epic stories can be parents",
            parent.story_type
        ));
    }
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::ParentEpicSet {
        meta,
        number,
        epic_number,
    })
    .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} grouped under epic #{epic_number}"))
}

pub(super) async fn handle_unset_parent_epic(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "unset parent epic")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let r = refresh.ok_or("unset_parent_epic requires refresh (RefreshDriver)")?;
    ensure_story_in_canonical_store(r, number).await?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::ParentEpicUnset { meta, number })
        .map_err(|e| format!("#{number} parent_epic-unset enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} ungrouped from parent epic"))
}

pub(super) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["ryan", "pm", "designer", "dev"], "accept")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::Accept)?;
    let r = refresh.ok_or(
        "accept_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    let role = role_from_name(agent_name);
    let signal = serde_json::to_string(&build_accept_signal(number, role))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    accept_via_canonical_store(r, number, actor, bypass).await?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} accepted and closed"))
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

pub(super) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["ryan", "pm", "designer", "dev"], "reject")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let r = refresh.ok_or(
        "reject_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("rejected")
        .to_string();
    let signal = serde_json::to_string(&build_reject_signal(number, &reason))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        crate::session::diag::log_diag(&format!("bridge: signal send failed: {e}"));
    }
    let actor = crate::transport::ws::short_name(agent_name).to_string();
    reject_via_canonical_store(r, number, actor, reason.clone()).await?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} rejected: {reason}"))
}

pub(crate) async fn reject_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    actor: String,
    reason: String,
) -> Result<(), String> {
    let prior = ensure_story_in_canonical_store(refresh, number).await?;
    let was_accepted = prior.is_closed || prior.state == crate::board::domain::StoryState::Accepted;
    let meta = crate::board::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    reap_workflow_labels(refresh.as_ref(), number, &meta, LABEL_READY)?;
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
            new_state: crate::board::domain::StoryState::Ready,
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
pub(super) async fn handle_pass_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["designer"], "pass desk-check")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::PassDeskCheck)?;
    let was_closed = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, number).await?;
        if let Some(body) = prior.body.as_deref() {
            crate::bridge::atp_substantive_gate::verify_atp_substantive(body)?;
        }
        prior.is_closed
    } else {
        false
    };
    // Pick signal recipients based on story type (Feature/Bug/Chore
    // per the 3-type vocabulary, #264). Chore + Bug → PM alone
    // (team-accept lane). Feature → Ryan AND PM (#259 — Ryan is the
    // acceptor, PM is the presenter; PM needs a direct signal so
    // presentation doesn't depend on scanning chat for designer's
    // PASS message).
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

/// Fail desk-check — designer only. Transitions DeskCheck → InProgress
/// (back-edge); rework-context lives in the gate-comment history, not
/// a separate state or label. Signal-send failure is fail-soft per the
/// start_story / accept_story / deliver_story precedent — state-
/// transition mutate_sync is load-bearing, signal is auxiliary.
pub(super) async fn handle_fail_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["designer"], "fail desk-check")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let was_accepted = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, number).await?;
        prior.is_closed || prior.state == crate::board::domain::StoryState::Accepted
    } else {
        false
    };
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
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
        reap_workflow_labels(r.as_ref(), number, &meta, LABEL_IN_PROGRESS)?;
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
            new_state: crate::board::domain::StoryState::InProgress,
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

/// PM-controlled priority reordering. For state="icebox", routes
/// through the canonical-via-GH label substrate (#1301 Option B per
/// team-converge); for other states, persists to legacy priority
/// index columns.
pub(super) async fn handle_reorder_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "reorder")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let position = args
        .get("position")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'position' argument")? as usize;
    let state_str = args
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'state' argument (e.g., 'ready', 'in_progress', 'delivered')")?;
    let state = crate::board::domain::StoryState::parse(state_str).ok_or_else(|| {
        format!("reorder_story state='{state_str}' not a valid StoryState (use icebox/ready/in-progress/desk-check/delivered/accepted)")
    })?;
    match state {
        crate::board::domain::StoryState::Icebox => {
            let r = refresh.ok_or(
                "reorder_story state=icebox requires refresh-driver (canonical-via-GH label substrate per #1301)",
            )?;
            reorder_icebox_story_via_labels(r, number, position)?;
            broadcast_board_invalidate(ws_tx).await;
            Ok(format!("#{number} moved to position {position} in icebox"))
        }
        crate::board::domain::StoryState::Ready => {
            let r = refresh.ok_or(
                "reorder_story state=ready requires refresh-driver (canonical-via-GH label substrate per #1301)",
            )?;
            reorder_ready_story_via_labels(r, number, position)?;
            broadcast_board_invalidate(ws_tx).await;
            Ok(format!("#{number} moved to position {position} in ready"))
        }
        _ => Err(format!(
            "reorder_story state='{}' not supported (#1301 retired per-state ordering — only 'ready' and 'icebox' are user-orderable)",
            state.as_str()
        )),
    }
}

pub(crate) fn reorder_icebox_story_via_labels(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    position: usize,
) -> Result<(), String> {
    let story = refresh
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in canonical store"))?;
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let current_iteration = story.raw_labels.iter().find_map(|l| {
        crate::board::iteration_label::parse_iteration_label(&codec, l)
            .map(|p| (p.theme_name, l.clone()))
    });
    let theme_name = current_iteration
        .as_ref()
        .map(|(t, _)| t.clone())
        .unwrap_or_else(|| "Unscheduled".to_string());
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| s.number != number && !s.is_closed)
        .filter_map(|s| {
            s.raw_labels.iter().find_map(|l| {
                crate::board::iteration_label::parse_iteration_label(&codec, l)
                    .filter(|p| p.theme_name == theme_name)
                    .map(|p| p.position)
            })
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position =
        crate::board::iteration_label::compute_insert_position(insert_index, &other_positions);
    let new_label = codec.encode_iteration(&theme_name, new_position);
    if let Some((_, current_label)) = &current_iteration
        && current_label.as_str() == new_label.as_str()
    {
        return Ok(());
    }
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    if let Some((_, current_label)) = current_iteration {
        refresh
            .mutate_sync(crate::board::BoardEvent::TagRemoved {
                meta: meta(),
                number,
                label: current_label,
            })
            .map_err(|e| format!("TagRemoved #{number}: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::TagAdded {
            meta: meta(),
            number,
            label: new_label,
        })
        .map_err(|e| format!("TagAdded #{number}: {e}"))?;
    Ok(())
}

pub(crate) fn reorder_ready_story_via_labels(
    refresh: &std::sync::Arc<dyn crate::board::RefreshDriver>,
    number: u64,
    position: usize,
) -> Result<(), String> {
    let story = refresh
        .story(number)
        .ok_or_else(|| format!("story #{number} not found in canonical store"))?;
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let current = story.raw_labels.iter().find_map(|l| {
        crate::board::ready_order_label::parse_ready_order_label(&codec, l).map(|p| (p, l.clone()))
    });
    let other_positions: Vec<u32> = refresh
        .all_stories()
        .iter()
        .filter(|s| {
            s.number != number && !s.is_closed && s.state == crate::board::domain::StoryState::Ready
        })
        .filter_map(|s| {
            s.raw_labels
                .iter()
                .find_map(|l| crate::board::ready_order_label::parse_ready_order_label(&codec, l))
        })
        .collect();
    let insert_index = position.saturating_sub(1).min(other_positions.len());
    let new_position =
        crate::board::iteration_label::compute_insert_position(insert_index, &other_positions);
    let new_label = codec.encode_ready_order(new_position);
    if let Some((_, current_label)) = &current
        && current_label.as_str() == new_label.as_str()
    {
        return Ok(());
    }
    let meta = || crate::board::events::EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    if let Some((_, current_label)) = current {
        refresh
            .mutate_sync(crate::board::BoardEvent::TagRemoved {
                meta: meta(),
                number,
                label: current_label,
            })
            .map_err(|e| format!("TagRemoved #{number}: {e}"))?;
    }
    refresh
        .mutate_sync(crate::board::BoardEvent::TagAdded {
            meta: meta(),
            number,
            label: new_label,
        })
        .map_err(|e| format!("TagAdded #{number}: {e}"))?;
    Ok(())
}

/// Add a comment to a story — any agent. Durable issue-audit-trail
/// (visible to any future reader of the issue; distinct from bpmux
/// chat which is session-scoped). Routes through the RefreshDriver
/// per #451; reject_story invokes the same path.
pub(super) async fn handle_comment_on_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("missing 'body' argument")?;
    if body.trim().is_empty() {
        return Err("'body' must not be empty".to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let event = crate::board::BoardEvent::FreeCommentAppended {
            meta: crate::board::events::EventMeta {
                actor: crate::transport::ws::short_name(agent_name).to_string(),
                ts: chrono::Utc::now(),
                peer_id: crate::board::PeerId::local(),
            },
            number,
            body: body.to_string(),
        };
        r.mutate_sync(event)
            .map_err(|e| format!("#{number} enqueue failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} commented"));
    }
    Err(
        "comment_on_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// Close a story with a reason — PM-only. Routes through the
/// RefreshDriver per #451 to append the reason as a comment, then
/// fire StoryClosed; accept_story uses the same close path.
/// Generalizes the invocation for superseded / duplicate / won't-fix
/// closures.
pub(super) async fn handle_close_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "close")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    if reason.trim().is_empty() {
        return Err("'reason' must not be empty — audit trail for closure".to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, number).await?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        reap_workflow_labels(r.as_ref(), number, &meta, KEEP_NONE)?;
        r.mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
        // #1151: drain dispatches StoryRefreshed from the close_story
        // tracker response, so the local store reflects post-close
        // state without a follow-up refresh_story round-trip.
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} close push failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} closed: {reason}"));
    }
    Err(
        "close_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// #530: replace a story's body via the canonical store. Closes the
/// gh-bypass class today's session repeatedly hit on ATP-corrections —
/// edits surface in the BoardStore + WS broadcast so every observer
/// sees the change without a separate refresh cycle.
pub(super) async fn handle_edit_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm", "ryan"], "edit story")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let body = args.get("body").and_then(|v| v.as_str());
    let title = args.get("title").and_then(|v| v.as_str());
    if body.is_none() && title.is_none() {
        return Err("edit_story requires at least one of 'body' or 'title' (#1279)".to_string());
    }
    let r = refresh
        .ok_or("edit_story requires refresh (RefreshDriver) — no canonical store available")?;
    let mut updated: Vec<&str> = Vec::new();
    if let Some(t) = title {
        r.update_title(number, t.to_string())
            .await
            .map_err(|e| format!("#{number} title update failed: {e}"))?;
        updated.push("title");
    }
    if let Some(b) = body {
        // #1151: the orchestrator's update_body now captures the
        // post-update Story from the adapter response and applies it to
        // the canonical store via StoryRefreshed. The follow-up
        // refresh_story round-trip is no longer needed.
        r.update_body(number, b.to_string())
            .await
            .map_err(|e| format!("#{number} body update failed: {e}"))?;
        updated.push("body");
    }
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} {} updated", updated.join(" + ")))
}

/// Fire the iteration-close externalize-prompt (#282). PM-gated.
///
/// Fail-soft: the MCP tool returns after the broadcast ws_send
/// completes. Iteration closes regardless of whether any agent
/// responds to the prompt — the tool is a reminder, not a barrier.
pub(super) async fn handle_close_iteration(
    agent_name: &str,
    args: &serde_json::Map<String, serde_json::Value>,
    ws_tx: &Mutex<Option<WsSink>>,
    port: u16,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "close iteration")?;
    let iteration_number = args.get("iteration_number").and_then(|v| v.as_u64());
    let outcome = post_close_iteration(port, iteration_number).await?;
    let signal = build_iteration_close_signal();
    let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, json).await {
        crate::session::diag::log_diag(&format!("bridge: iteration-close signal send failed: {e}"));
    }
    Ok(format!(
        "iteration-close signal broadcast — agents prompted to flush unsaved findings. \
         promoted: {} stories; deleted marker: {}",
        outcome.promoted.len(),
        outcome
            .deleted_marker_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none (no markers in icebox)".to_string()),
    ))
}

#[derive(Debug, Default, serde::Deserialize)]
struct CloseIterationOutcomeJson {
    #[serde(default)]
    promoted: Vec<u64>,
    #[serde(default)]
    deleted_marker_id: Option<u64>,
}

async fn post_close_iteration(
    port: u16,
    iteration_number: Option<u64>,
) -> Result<CloseIterationOutcomeJson, String> {
    let token_path = crate::session::auth_token::default_auth_token_path()
        .ok_or_else(|| "cannot resolve auth-token path (HOME unset?)".to_string())?;
    let token = crate::session::auth_token::read(&token_path)
        .map_err(|e| format!("read auth-token from {}: {e}", token_path.display()))?;
    let url = crate::transport::local_url::api_url(port, "/api/close_iteration");
    let body = serde_json::json!({ "iteration_number": iteration_number });
    let res = reqwest::Client::new()
        .post(&url)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("close_iteration: bpmux-server unreachable at {url}: {e}"))?;
    let status = res.status();
    let text = res
        .text()
        .await
        .map_err(|e| format!("close_iteration: read body: {e}"))?;
    if !status.is_success() {
        return Err(format!("close_iteration: {status} — {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("close_iteration: parse outcome: {e}"))
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

pub(super) fn format_bypass_payload(reason: String) -> String {
    format!("[bug-question bypass] {reason}")
}
