//! Story management tool handlers (#140, #194, #197).
//!
//! Lane-aware operations on GitHub issues via the tracker trait
//! (octocrab-backed per #320). Workflow signals fire on INTENT (before
//! the tracker call) so the team is notified even when the mutation
//! fails.

use tokio::sync::Mutex;

use super::tools::{WsSink, ws_send};
use crate::transport::protocol::ClientMessage;

/// GitHub label vocabulary. Centralized so the read-checks
/// (handle_view_board) and the write-calls (handle_start/deliver/etc.)
/// stay in lockstep — a typo in one and not the other would silently
/// misroute a story in the board view. New labels go here first, call
/// sites reference the const.
const LABEL_DELIVERED: &str = "delivered";

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

/// Pins-update broadcast marker (#342). Emitted when a viewer pins
/// or unpins a message; peer renderers subscribe and refetch pins
/// for the affected channel so everyone sees the same pin state
/// within WS latency.
pub const PINS_UPDATE_MARKER: &str = "__BPMUX_PINS_UPDATE__";

/// Reactions-update broadcast marker (#340). Same pattern as
/// PINS_UPDATE_MARKER but for emoji reactions — peer renderers
/// refetch the channel's aggregated reactions on receipt.
pub const REACTIONS_UPDATE_MARKER: &str = "__BPMUX_REACTIONS_UPDATE__";

/// CI-state-change broadcast marker (#372). Emitted by the server's
/// background CI poller when the current conclusion on main differs
/// from the last-seen conclusion. The renderer's CiGlyph subscribes
/// and refetches `/api/ci` immediately, so the user sees state
/// transitions within WS latency instead of waiting for the 15s
/// client-side poll tick (#361). The 15s poll stays as a safety net
/// for the disconnected-then-reconnected window.
pub const CI_STATE_UPDATE_MARKER: &str = "__BPMUX_CI_STATE_UPDATE__";

/// #402 team-roster substrate. Broadcast when a WS peer joins /
/// leaves / renames so the frontend's `useTeamRoster` hook refetches
/// `/api/roster`. Push-based membership updates; same pattern as the
/// board-invalidate + ci-state markers.
pub const ROSTER_UPDATE_MARKER: &str = "__BPMUX_ROSTER_UPDATE__";

/// Build the board-invalidate broadcast signal. System-typed so the
/// TUI recognizes it as an internal marker rather than a chat line.
/// No `to=` set — every agent's TUI receives + refreshes.
pub(super) fn build_board_invalidate_signal() -> ClientMessage {
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
pub(super) async fn broadcast_board_invalidate(ws_tx: &Mutex<Option<WsSink>>) {
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

pub(super) fn role_from_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

pub(super) fn require_role(
    agent_name: &str,
    allowed: &[&str],
    operation: &str,
) -> Result<(), String> {
    let role = role_from_name(agent_name);
    if allowed.contains(&role) {
        Ok(())
    } else {
        Err(format!(
            "{operation} is restricted to {}. ask {} to {operation}.",
            allowed.join(" or "),
            allowed[0],
        ))
    }
}

pub(super) async fn handle_view_board(
    _args: &serde_json::Map<String, serde_json::Value>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let r = refresh.ok_or(
        "view_board requires refresh (RefreshDriver) — tracker fallback retired in slice 5l",
    )?;
    let stories = r.all_stories();
    let format_entry = |s: &crate::board::domain::Story| -> (u64, String) {
        let prefix = if s.parent_epic.is_some() { "↳ " } else { "" };
        let suffix = match s.parent_epic {
            Some(n) => format!(" (epic #{n})"),
            None => String::new(),
        };
        (
            s.number,
            format!("{prefix}#{} {}{}", s.number, s.title, suffix),
        )
    };
    let bucket_for = |state: crate::board::domain::StoryState| -> Vec<String> {
        let mut entries: Vec<(u64, String)> = stories
            .iter()
            .filter(|s| s.story_type != crate::board::domain::StoryType::Epic)
            .filter(|s| s.state == state)
            .filter(|s| !s.is_closed || s.state == crate::board::domain::StoryState::Accepted)
            .map(format_entry)
            .collect();
        entries.sort_by_key(|(n, _)| *n);
        entries.into_iter().map(|(_, e)| e).collect()
    };
    let icebox = bucket_for(crate::board::domain::StoryState::Icebox);
    let ready = bucket_for(crate::board::domain::StoryState::Ready);
    let in_progress = bucket_for(crate::board::domain::StoryState::InProgress);
    let delivered = bucket_for(crate::board::domain::StoryState::Delivered);
    let ready_for_accept = bucket_for(crate::board::domain::StoryState::ReadyForAccept);
    let needs_fixes = bucket_for(crate::board::domain::StoryState::NeedsFixes);
    let rejected = bucket_for(crate::board::domain::StoryState::Rejected);
    let accepted = bucket_for(crate::board::domain::StoryState::Accepted);
    let mut epics: Vec<&crate::board::domain::Story> = stories
        .iter()
        .filter(|s| s.story_type == crate::board::domain::StoryType::Epic)
        .collect();
    epics.sort_by_key(|s| s.number);
    let mut out = String::new();
    if !epics.is_empty() {
        out.push_str(&format!("EPICS ({})\n", epics.len()));
        for epic in &epics {
            let children: Vec<&crate::board::domain::Story> = stories
                .iter()
                .filter(|c| c.parent_epic == Some(epic.number))
                .collect();
            let total = children.len();
            let accepted_count = children
                .iter()
                .filter(|c| c.state == crate::board::domain::StoryState::Accepted)
                .count();
            let delivered_count = children
                .iter()
                .filter(|c| c.state == crate::board::domain::StoryState::Delivered)
                .count();
            let in_progress_count = children
                .iter()
                .filter(|c| c.state == crate::board::domain::StoryState::InProgress)
                .count();
            let derived = crate::board::domain::derive_epic_state(epic.state, &children);
            out.push_str(&format!(
                "  #{} [{}] {} ({total} children: {accepted_count} accepted, {delivered_count} delivered, {in_progress_count} in-progress)\n",
                epic.number, derived.as_str(), epic.title
            ));
        }
        out.push('\n');
    }
    let section = |name: &str, items: &[String], out: &mut String| {
        out.push_str(&format!("{name} ({})\n", items.len()));
        for item in items {
            out.push_str(&format!("  {item}\n"));
        }
        out.push('\n');
    };
    section("ICEBOX", &icebox, &mut out);
    section("READY", &ready, &mut out);
    section("IN PROGRESS", &in_progress, &mut out);
    section("DELIVERED", &delivered, &mut out);
    section("READY FOR ACCEPT", &ready_for_accept, &mut out);
    section("NEEDS FIXES", &needs_fixes, &mut out);
    section("REJECTED", &rejected, &mut out);
    section("ACCEPTED", &accepted, &mut out);
    Ok(out)
}

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
    // Assemble the final body with structured header sections so the
    // user_job + falsifier land in the GitHub issue as first-class
    // content (renderable at filing-gate; auditable afterward).
    // Author-supplied `body` appends below the structured header.
    let author_body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let composed_body = compose_story_body(user_job, observation_falsifier, author_body);
    let parent_epic = args.get("parent_epic").and_then(|v| v.as_u64());
    if let Some(r) = refresh {
        if let Some(epic_number) = parent_epic {
            r.refresh_story(epic_number)
                .await
                .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
            let parent = r.story(epic_number).ok_or_else(|| {
                format!("epic #{epic_number} not found — cannot set as parent at filing")
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

fn require_nonempty_string<'a>(
    args: &'a serde_json::Map<String, serde_json::Value>,
    key: &str,
    error_doc: &str,
) -> Result<&'a str, String> {
    let val = args.get(key).and_then(|v| v.as_str()).unwrap_or("").trim();
    if val.is_empty() {
        return Err(error_doc.to_string());
    }
    // Return the original untrimmed slice so downstream formatting
    // preserves intentional leading/trailing newlines in multi-line
    // field values.
    let raw = args.get(key).and_then(|v| v.as_str()).unwrap_or_default();
    Ok(raw)
}

fn compose_story_body(user_job: &str, observation_falsifier: &str, author_body: &str) -> String {
    let mut out = String::new();
    out.push_str("## User job\n\n");
    out.push_str(user_job.trim());
    out.push_str("\n\n## Observation falsifier\n\n");
    out.push_str(observation_falsifier.trim());
    out.push_str("\n\n---\n\n");
    out.push_str(author_body);
    out
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
    if let Some(r) = refresh {
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    }
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
        Some(r) => Some(
            r.refresh_story(number)
                .await
                .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?,
        ),
        None => None,
    };
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
        ws_send(ws_tx, json)
            .await
            .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    }
    // #391: persist the verified_observation as a comment on the
    // story so designer + pm see it at desk-check + accept gates.
    // Comment is auditable; fits the existing comment substrate.
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
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
            new_state: crate::board::domain::StoryState::Delivered,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
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
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
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
    r.refresh_story(number)
        .await
        .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    r.refresh_story(epic_number)
        .await
        .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    let parent = r.story(epic_number).ok_or_else(|| {
        format!("epic #{epic_number} not found — cannot set as parent of #{number}")
    })?;
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
    r.refresh_story(number)
        .await
        .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
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
    if let Some(r) = refresh {
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    }
    let role = role_from_name(agent_name);
    let signal = serde_json::to_string(&build_accept_signal(number, role))
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
        r.mutate_sync(crate::board::BoardEvent::TagRemoved {
            meta: meta.clone(),
            number,
            label: LABEL_DELIVERED.to_string(),
        })
        .map_err(|e| format!("#{number} delivered-tag-remove enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Accepted,
            payload: None,
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta: meta.clone(),
            number,
            new_state: crate::board::domain::StoryState::Accepted,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: None,
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} accepted and closed"));
    }
    Err(
        "accept_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
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
    if let Some(r) = refresh {
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    }
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("rejected");
    let signal = serde_json::to_string(&build_reject_signal(number, reason))
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
        r.mutate_sync(crate::board::BoardEvent::TagRemoved {
            meta: meta.clone(),
            number,
            label: LABEL_DELIVERED.to_string(),
        })
        .map_err(|e| format!("#{number} delivered-tag-remove enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::Rejected,
            payload: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::Rejected,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} rejected: {reason}"));
    }
    Err(
        "reject_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
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
    if let Some(r) = refresh {
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    }
    // Pick signal recipients based on story type (Feature/Bug/Chore
    // per the 3-type vocabulary, #264). Chore + Bug → PM alone
    // (team-accept lane). Feature → Ryan AND PM (#259 — Ryan is the
    // acceptor, PM is the presenter; PM needs a direct signal so
    // presentation doesn't depend on scanning chat for designer's
    // PASS message).
    let story_type = classify_for_pass_desk_check(number, refresh);
    for msg in build_pass_desk_check_signals(number, story_type) {
        let signal = serde_json::to_string(&msg).map_err(|e| format!("signal: {e}"))?;
        ws_send(ws_tx, signal)
            .await
            .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    }
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::PassDeskCheck,
            payload: None,
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::ReadyForAccept,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
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

/// Fail desk-check — designer only. Signals dev that the story needs
/// fixes (applies the `needs fixes` label). Same Err-vs-Ok split as
/// pass_desk_check: Err on signal send failure, Ok with gh status.
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
    if let Some(r) = refresh {
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
    }
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let signal = serde_json::to_string(&build_fail_desk_check_signal(number, reason))
        .map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, signal)
        .await
        .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
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
            new_state: crate::board::domain::StoryState::NeedsFixes,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
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

/// PM-controlled priority reordering; persists to priority index.
pub(super) async fn handle_reorder_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
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
    let state = args
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'state' argument (e.g., 'ready', 'in_progress', 'delivered')")?;

    let path = crate::session::priority::default_priority_path()
        .map_err(|e| format!("priority path: {e}"))?;
    let mut index = crate::session::priority::load_priority(&path);
    crate::session::priority::reorder(&mut index, state, number, position);
    crate::session::priority::save_priority(&path, &index)
        .map_err(|e| format!("save priority: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("#{number} moved to position {position} in {state}"))
}

/// Add a comment to a story — any agent. Durable issue-audit-trail
/// (visible to any future reader of the issue; distinct from bpmux
/// chat which is session-scoped). Thin wrapper on existing
/// `tracker.comment_on_story`; capability is pre-existing
/// (reject_story invokes the same path), this generalizes for
/// standalone commenting.
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
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
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

/// Close a story with a reason — PM-only. Adds the reason as a
/// comment on the issue, then closes it. Thin wrapper on existing
/// `tracker.comment_on_story` + `tracker.close_story` — capability
/// is pre-existing (accept_story uses the same close path), this
/// generalizes the invocation for superseded / duplicate / won't-fix
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
        r.refresh_story(number)
            .await
            .map_err(|e| format!("source unreachable on refresh-before-write: {e}"))?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        r.mutate_sync(crate::board::BoardEvent::StoryClosed {
            meta,
            number,
            reason: Some(reason.to_string()),
        })
        .map_err(|e| format!("#{number} close enqueue failed: {e}"))?;
        broadcast_board_invalidate(ws_tx).await;
        return Ok(format!("#{number} closed: {reason}"));
    }
    Err(
        "close_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

/// Fire the iteration-close externalize-prompt (#282). PM-gated.
///
/// Fail-soft: the MCP tool returns after the broadcast ws_send
/// completes. Iteration closes regardless of whether any agent
/// responds to the prompt — the tool is a reminder, not a barrier.
pub(super) async fn handle_close_iteration(
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(agent_name, &["pm"], "close iteration")?;
    let signal = build_iteration_close_signal();
    let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, json)
        .await
        .map_err(|e| format!("iteration-close signal send failed: {e}"))?;
    Ok("iteration-close signal broadcast — agents prompted to flush unsaved findings".to_string())
}

// ── Workflow signal builders (#197) ──────────────────────────

pub(super) fn build_delivery_signals(number: u64, title: &str) -> Vec<ClientMessage> {
    vec![
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered — desk-check needed"),
            to: Some(vec!["designer".to_string()]),
            addressee: Some("designer".to_string()),
            message_type: None,
        },
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered. queue update: check ready count"),
            to: Some(vec!["pm".to_string()]),
            addressee: Some("pm".to_string()),
            message_type: None,
        },
    ]
}

pub(super) fn build_start_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} started. ready queue shrunk — check if restocking needed"),
        to: Some(vec!["pm".to_string()]),
        addressee: Some("pm".to_string()),
        message_type: None,
    }
}

pub(super) fn build_accept_signal(number: u64, accepted_by: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!(
            "#{number} accepted by {accepted_by}. pick the next story from the ready queue"
        ),
        to: None,
        addressee: Some("dev".to_string()),
        message_type: None,
    }
}

pub(super) fn build_reject_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} rejected: {reason}. fix and re-deliver"),
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
pub(super) fn classify_for_pass_desk_check(
    number: u64,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> crate::board::domain::StoryType {
    refresh
        .and_then(|r| r.story(number))
        .map(|s| s.story_type)
        .unwrap_or(crate::board::domain::StoryType::Feature)
}

pub(super) fn addressee_for_pass_desk_check(
    story_type: crate::board::domain::StoryType,
) -> &'static str {
    match story_type {
        crate::board::domain::StoryType::Feature | crate::board::domain::StoryType::Epic => "ryan",
        crate::board::domain::StoryType::Bug | crate::board::domain::StoryType::Chore => "pm",
    }
}

pub(super) fn build_pass_desk_check_signal(number: u64, addressee: &str) -> ClientMessage {
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
pub(super) fn build_pass_desk_check_pm_present_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} passed designer desk-check. present to ryan."),
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
pub(super) fn build_pass_desk_check_signals(
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

pub(super) fn build_fail_desk_check_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} failed desk-check: {reason} — needs fixes."),
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
pub(super) fn build_iteration_close_signal() -> ClientMessage {
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
