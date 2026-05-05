//! `move_story` — unified state-traversal MCP tool (#1188).
//!
//! Single tool consolidating the existing label_story / reorder_story /
//! close_story chains. Atomic at canonical-store level: refresh →
//! reap stale workflow labels → fire StateTransitioned (and StoryReopened
//! / StoryClosed for cross-terminal cases) → optionally reorder →
//! append audit-trail comment → broadcast invalidate. No half-states
//! observable per #1188 falsifier-h.
//!
//! Permission boundary (falsifier-e + scope-question 1):
//! - any role can move a story across COLUMNS (state-transition) —
//!   reflects "agents are users" + Ryan 01:54 "agent puts a story
//!   wherever it needs to be";
//! - position is honored only when target_state is Ready or Icebox
//!   (the queue-organization columns) AND the caller is PM —
//!   reconciles with Ryan 01:39 "PM feeds and organizes the ready
//!   queue".
//!
//! Terminal-state recovery (falsifier-c + scope-question 2):
//! moving a story OUT of Accepted (or any closed state) into a non-
//! terminal state fires StoryReopened first, then the workflow-label
//! transition. Closes the #1177-stuck class structurally.
//!
//! Audit-trail (falsifier-f): every move appends a FreeCommentAppended
//! capturing actor + from_state + to_state + position + rationale + ts.
//! Issue-comment is the persistent surface; story-timeline derives
//! from comment audit-trail.
//!
//! Goes through `RefreshDriver::mutate_sync` per #1170 lesson
//! (`feedback_rest_mcp_canonical_mutation_parity`). No direct adapter
//! writes — all paths are reducer-cascading so observers (BoardStore,
//! TUI, Electron renderer) see consistent post-state.

use std::sync::Arc;

use serde_json::Map;
use serde_json::Value;
use tokio::sync::Mutex;

use crate::board::RefreshDriver;
use crate::board::domain::StoryState;
use crate::board::events::EventMeta;
use crate::bridge::tools_stories::{
    KEEP_NONE, broadcast_board_invalidate, reap_workflow_labels, require_role,
};

use super::tools::WsSink;

/// Reserved name for the closed/retired terminal state. Not a
/// `StoryState` variant because the canonical model treats it as
/// "issue is closed" — a separate axis from workflow_state per
/// `feedback_state_transitions_distinct_dont_conflate`. We accept it
/// as a target_state argument and translate to BoardEvent::StoryClosed.
const TARGET_STATE_RETIRED: &str = "retired";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetState {
    Workflow(StoryState),
    Retired,
}

impl TargetState {
    fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "icebox" => Ok(Self::Workflow(StoryState::Icebox)),
            "ready" => Ok(Self::Workflow(StoryState::Ready)),
            "in-progress" | "in_progress" | "in progress" => {
                Ok(Self::Workflow(StoryState::InProgress))
            }
            "desk-check" | "desk_check" | "deskcheck" => Ok(Self::Workflow(StoryState::DeskCheck)),
            "delivered" => Ok(Self::Workflow(StoryState::Delivered)),
            "accepted" => Ok(Self::Workflow(StoryState::Accepted)),
            TARGET_STATE_RETIRED => Ok(Self::Retired),
            other => Err(format!(
                "unknown target_state '{other}'. Valid: icebox, ready, in-progress, desk-check, delivered, accepted, retired"
            )),
        }
    }

    fn as_label(self) -> Option<&'static str> {
        match self {
            Self::Workflow(StoryState::Ready) => Some("ready"),
            Self::Workflow(StoryState::InProgress) => Some("in progress"),
            Self::Workflow(StoryState::DeskCheck) => Some("desk-check"),
            Self::Workflow(StoryState::Delivered) => Some("delivered"),
            Self::Workflow(StoryState::Accepted) => Some("accepted"),
            Self::Workflow(StoryState::Icebox) | Self::Retired => None,
        }
    }

    fn closes_issue(self) -> bool {
        matches!(self, Self::Retired | Self::Workflow(StoryState::Accepted))
    }

    fn as_display(self) -> &'static str {
        match self {
            Self::Workflow(StoryState::Icebox) => "icebox",
            Self::Workflow(StoryState::Ready) => "ready",
            Self::Workflow(StoryState::InProgress) => "in-progress",
            Self::Workflow(StoryState::DeskCheck) => "desk-check",
            Self::Workflow(StoryState::Delivered) => "delivered",
            Self::Workflow(StoryState::Accepted) => "accepted",
            Self::Retired => "retired",
        }
    }
}

/// Falsifier-d disambiguation — position is an absolute 1-based index
/// within the destination column's priority list. v1 doesn't support
/// "before #N" / "after #N" semantics; the spec leaves this as a
/// future extension once we observe a use case the absolute form
/// can't satisfy.
fn parse_position(args: &Map<String, Value>) -> Result<Option<usize>, String> {
    match args.get("position") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_u64()
                .ok_or("'position' must be a 1-based integer or null")?;
            if n == 0 {
                return Err("'position' is 1-based; got 0".to_string());
            }
            Ok(Some(n as usize))
        }
    }
}

fn require_nonempty(args: &Map<String, Value>, key: &str, hint: &str) -> Result<String, String> {
    let value = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing '{key}' argument; {hint}"))?
        .trim()
        .to_string();
    if value.is_empty() {
        return Err(format!("'{key}' must not be blank; {hint}"));
    }
    Ok(value)
}

fn current_state_display(story: Option<&crate::board::domain::Story>) -> &'static str {
    match story {
        Some(s) if s.is_closed => "retired-or-accepted",
        Some(s) => match s.state {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in-progress",
            StoryState::DeskCheck => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Accepted => "accepted",
        },
        None => "unknown",
    }
}

/// Build the audit-trail comment body. Includes actor (the role that
/// fired the move), the from→to transition, optional new position,
/// and the rationale. This is the substrate behind story-timeline
/// per falsifier-f — view_story_timeline reads from comment events
/// using the same shape.
fn audit_comment_body(
    actor_short: &str,
    from_state: &str,
    target: TargetState,
    position: Option<usize>,
    rationale: &str,
) -> String {
    let pos_part = match position {
        Some(p) => format!(" position={p}"),
        None => String::new(),
    };
    format!(
        "**move_story** by {actor_short}: {from_state} → {to}{pos_part}\n\n{rationale}",
        to = target.as_display(),
    )
}

pub(in crate::bridge) async fn handle_move_story(
    args: &Map<String, Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&Arc<dyn RefreshDriver>>,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let target_str = args
        .get("target_state")
        .and_then(|v| v.as_str())
        .ok_or("missing 'target_state' argument")?;
    let target = TargetState::parse(target_str)?;
    let position = parse_position(args)?;
    let rationale = require_nonempty(
        args,
        "rationale",
        "rationale is required (#1188 falsifier-b — every move must surface an audit-trail reason)",
    )?;

    if position.is_some() {
        let allows_position = matches!(
            target,
            TargetState::Workflow(StoryState::Ready) | TargetState::Workflow(StoryState::Icebox)
        );
        if !allows_position {
            return Err(format!(
                "'position' only meaningful for ready/icebox columns; target_state={} ignores ordering",
                target.as_display()
            ));
        }
        require_role(
            agent_name,
            &["pm"],
            "specify position in ready/icebox (PM owns queue-organization per Ryan 2026-05-05 01:39 UTC)",
        )?;
    }

    let r = refresh
        .ok_or("move_story requires refresh (RefreshDriver) — tracker fallback not supported")?;
    // #1194: read prior state from canonical store; refresh_story
    // removed per "MCP actions only pull from / mutate the store"
    // (Ryan 2026-05-05 12:57 UTC). Ingest poller keeps the store
    // ≤60s fresh; outbox drain propagates this mutation to GH.
    let prior = r.story(number);
    let from_display = current_state_display(prior.as_ref()).to_string();
    let was_closed = prior.as_ref().map(|s| s.is_closed).unwrap_or(false);

    let actor_short = crate::transport::ws::short_name(agent_name).to_string();
    let meta = EventMeta {
        actor: actor_short.clone(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };

    if was_closed && !target.closes_issue() {
        r.mutate_sync(crate::board::BoardEvent::StoryReopened {
            meta: meta.clone(),
            number,
        })
        .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
    }

    let keep_label = target.as_label().unwrap_or(KEEP_NONE);
    reap_workflow_labels(r.as_ref(), number, &meta, keep_label)?;

    if let Some(label) = target.as_label() {
        let already_labeled = prior
            .as_ref()
            .map(|s| s.raw_labels.iter().any(|l| l == label))
            .unwrap_or(false);
        if !already_labeled {
            r.mutate_sync(crate::board::BoardEvent::TagAdded {
                meta: meta.clone(),
                number,
                label: label.to_string(),
            })
            .map_err(|e| format!("#{number} tag-add enqueue failed: {e}"))?;
        }
    }

    match target {
        TargetState::Workflow(state) => {
            r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
                meta: meta.clone(),
                number,
                new_state: state,
            })
            .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
            if matches!(state, StoryState::Accepted) && !was_closed {
                r.mutate_sync(crate::board::BoardEvent::StoryClosed {
                    meta: meta.clone(),
                    number,
                    reason: Some(rationale.clone()),
                })
                .map_err(|e| format!("#{number} accept-close enqueue failed: {e}"))?;
            }
        }
        TargetState::Retired => {
            r.mutate_sync(crate::board::BoardEvent::StoryClosed {
                meta: meta.clone(),
                number,
                reason: Some(format!("retired: {rationale}")),
            })
            .map_err(|e| format!("#{number} retire enqueue failed: {e}"))?;
            // #1192: stamp an explicit "retired" label so state_from_
            // labels can distinguish closed-via-retire from closed-via-
            // accept on the next refresh. Without this, the canonical
            // store collapses both into StoryState::Accepted and the
            // renderer's Retired column never receives the story.
            let already_retired = prior
                .as_ref()
                .map(|s| {
                    s.raw_labels
                        .iter()
                        .any(|l| l.eq_ignore_ascii_case("retired"))
                })
                .unwrap_or(false);
            if !already_retired {
                r.mutate_sync(crate::board::BoardEvent::TagAdded {
                    meta: meta.clone(),
                    number,
                    label: "retired".to_string(),
                })
                .map_err(|e| format!("#{number} retire-tag enqueue failed: {e}"))?;
            }
        }
    }

    if let (Some(pos), Some(state_str)) = (position, target_position_state_label(target)) {
        let path = crate::session::priority::default_priority_path()
            .map_err(|e| format!("priority path: {e}"))?;
        let mut index = crate::session::priority::load_priority(&path);
        crate::session::priority::reorder(&mut index, state_str, number, pos);
        crate::session::priority::save_priority(&path, &index)
            .map_err(|e| format!("save priority: {e}"))?;
    }

    let body = audit_comment_body(&actor_short, &from_display, target, position, &rationale);
    r.mutate_sync(crate::board::BoardEvent::FreeCommentAppended { meta, number, body })
        .map_err(|e| format!("#{number} audit-comment enqueue failed: {e}"))?;

    broadcast_board_invalidate(ws_tx).await;

    let pos_msg = match position {
        Some(p) => format!(" at position {p}"),
        None => String::new(),
    };
    Ok(format!(
        "#{number} moved {from_display} → {to}{pos_msg}",
        to = target.as_display()
    ))
}

fn target_position_state_label(target: TargetState) -> Option<&'static str> {
    match target {
        TargetState::Workflow(StoryState::Ready) => Some("ready"),
        TargetState::Workflow(StoryState::Icebox) => Some("icebox"),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn target_state_parses_canonical_spellings() {
        assert_eq!(
            TargetState::parse("ready").unwrap(),
            TargetState::Workflow(StoryState::Ready)
        );
        assert_eq!(
            TargetState::parse("In-Progress").unwrap(),
            TargetState::Workflow(StoryState::InProgress)
        );
        assert_eq!(TargetState::parse("retired").unwrap(), TargetState::Retired);
        assert_eq!(
            TargetState::parse("Accepted").unwrap(),
            TargetState::Workflow(StoryState::Accepted)
        );
    }

    #[test]
    fn target_state_parse_unknown_lists_valid_options() {
        let err = TargetState::parse("bogus").unwrap_err();
        assert!(
            err.contains("retired") && err.contains("ready") && err.contains("delivered"),
            "error must enumerate valid options for the agent: got {err}"
        );
    }

    #[test]
    fn closes_issue_is_true_only_for_terminal_states() {
        assert!(TargetState::Retired.closes_issue());
        assert!(TargetState::Workflow(StoryState::Accepted).closes_issue());
        assert!(!TargetState::Workflow(StoryState::Delivered).closes_issue());
        assert!(!TargetState::Workflow(StoryState::Ready).closes_issue());
        assert!(!TargetState::Workflow(StoryState::Icebox).closes_issue());
    }

    #[test]
    fn parse_position_rejects_zero_and_non_integer() {
        let mut args: Map<String, Value> = Map::new();
        args.insert("position".to_string(), serde_json::json!(0));
        assert!(parse_position(&args).is_err());
        args.insert("position".to_string(), serde_json::json!("first"));
        assert!(parse_position(&args).is_err());
    }

    #[test]
    fn parse_position_returns_none_when_absent_or_null() {
        let mut args: Map<String, Value> = Map::new();
        assert_eq!(parse_position(&args).unwrap(), None);
        args.insert("position".to_string(), Value::Null);
        assert_eq!(parse_position(&args).unwrap(), None);
    }

    #[test]
    fn parse_position_accepts_one_based_positive_integer() {
        let mut args: Map<String, Value> = Map::new();
        args.insert("position".to_string(), serde_json::json!(1));
        assert_eq!(parse_position(&args).unwrap(), Some(1));
        args.insert("position".to_string(), serde_json::json!(42));
        assert_eq!(parse_position(&args).unwrap(), Some(42));
    }

    #[test]
    fn require_nonempty_rejects_blank_or_whitespace_only() {
        let mut args: Map<String, Value> = Map::new();
        args.insert("rationale".to_string(), Value::String("".to_string()));
        assert!(require_nonempty(&args, "rationale", "hint").is_err());
        args.insert(
            "rationale".to_string(),
            Value::String("   \t\n".to_string()),
        );
        assert!(require_nonempty(&args, "rationale", "hint").is_err());
    }

    #[test]
    fn require_nonempty_trims_returned_value() {
        let mut args: Map<String, Value> = Map::new();
        args.insert(
            "rationale".to_string(),
            Value::String("  cleanup before iteration close  ".to_string()),
        );
        let v = require_nonempty(&args, "rationale", "hint").unwrap();
        assert_eq!(v, "cleanup before iteration close");
    }

    #[test]
    fn audit_comment_body_includes_actor_transition_position_rationale() {
        let body = audit_comment_body(
            "dev",
            "delivered",
            TargetState::Workflow(StoryState::Ready),
            Some(3),
            "rework after desk-check",
        );
        assert!(body.contains("**move_story** by dev"));
        assert!(body.contains("delivered → ready"));
        assert!(body.contains("position=3"));
        assert!(body.contains("rework after desk-check"));
    }

    #[test]
    fn audit_comment_body_omits_position_segment_when_none() {
        let body = audit_comment_body(
            "pm",
            "icebox",
            TargetState::Workflow(StoryState::Ready),
            None,
            "next-iteration promote",
        );
        assert!(body.contains("icebox → ready"));
        assert!(
            !body.contains("position="),
            "absent position must not surface a position= segment in the audit body"
        );
    }

    #[test]
    fn target_position_state_label_only_returns_for_ready_and_icebox() {
        assert_eq!(
            target_position_state_label(TargetState::Workflow(StoryState::Ready)),
            Some("ready")
        );
        assert_eq!(
            target_position_state_label(TargetState::Workflow(StoryState::Icebox)),
            Some("icebox")
        );
        assert_eq!(
            target_position_state_label(TargetState::Workflow(StoryState::InProgress)),
            None
        );
        assert_eq!(target_position_state_label(TargetState::Retired), None);
    }
}
