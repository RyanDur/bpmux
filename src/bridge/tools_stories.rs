//! Story management tool handlers (#140, #194, #197).
//!
//! Lane-aware operations on GitHub issues via server-side `gh` proxy.
//! Workflow signals fire on INTENT (before gh) so the team is notified
//! even when gh fails.

use tokio::sync::Mutex;

use super::tools::{WsSink, ws_send};
use crate::transport::protocol::ClientMessage;

/// Compile-time repo root — guaranteed to be the bpmux source
/// directory regardless of which workspace starts the bridge.
const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// GitHub label vocabulary. Centralized so the read-checks
/// (handle_view_board) and the write-calls (handle_start/deliver/etc.)
/// stay in lockstep — a typo in one and not the other would silently
/// misroute a story in the board view. New labels go here first, call
/// sites reference the const.
const LABEL_READY: &str = "ready";
const LABEL_IN_PROGRESS: &str = "in progress";
const LABEL_DELIVERED: &str = "delivered";
const LABEL_DEFERRED: &str = "deferred";
const LABEL_STARTED: &str = "started";
const LABEL_FINISHED: &str = "finished";
const LABEL_REJECTED: &str = "rejected";
const LABEL_READY_FOR_ACCEPT: &str = "ready for accept";
const LABEL_NEEDS_FIXES: &str = "needs fixes";

/// Run a gh command and return stdout. Errors include stderr.
/// Sets the working directory to the bpmux repo so `gh` can find
/// git remotes even when the bridge runs from an agent workspace.
pub(crate) async fn gh(args: &[&str]) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("gh");
    cmd.args(args);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    } else {
        cmd.current_dir(REPO_ROOT);
    }
    let output = tokio::time::timeout(std::time::Duration::from_secs(30), cmd.output())
        .await
        .map_err(|_| "gh command timed out after 30s".to_string())?
        .map_err(|e| format!("gh: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    let json_str = tracker.list_stories().await?;

    let issues: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| format!("parse: {e}"))?;

    let mut ready = Vec::new();
    let mut in_progress = Vec::new();
    let mut delivered = Vec::new();
    let mut accepted = Vec::new();
    let mut deferred = Vec::new();
    let mut icebox = Vec::new();

    for issue in &issues {
        let num = issue["number"].as_u64().unwrap_or(0);
        let title = issue["title"].as_str().unwrap_or("?");
        let gh_state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        let entry = format!("#{num} {title}");
        let has = |name: &str| labels.contains(&name);

        if gh_state == "CLOSED" {
            accepted.push(entry);
        } else if has(LABEL_DEFERRED) {
            deferred.push(entry);
        } else if has(LABEL_DELIVERED) {
            delivered.push(entry);
        } else if has(LABEL_STARTED) || has(LABEL_FINISHED) {
            in_progress.push(entry);
        } else if has(LABEL_READY) {
            ready.push(entry);
        } else {
            icebox.push(entry);
        }
    }

    let mut out = String::new();
    let section = |name: &str, items: &[String], out: &mut String| {
        out.push_str(&format!("{name} ({})\n", items.len()));
        for item in items {
            out.push_str(&format!("  {item}\n"));
        }
        out.push('\n');
    };
    section("READY", &ready, &mut out);
    section("IN PROGRESS", &in_progress, &mut out);
    section("DELIVERED", &delivered, &mut out);
    section("ACCEPTED", &accepted, &mut out);
    section("DEFERRED", &deferred, &mut out);
    section("ICEBOX", &icebox, &mut out);
    Ok(out)
}

pub(super) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let result = tracker.create_story(title, body).await?;
    Ok(format!("story created: {}", result.trim()))
}

/// #197: signal fires on INTENT (before gh) so the team is notified
/// even when the gh label edit fails.
pub(super) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "start")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let signal =
        serde_json::to_string(&build_start_signal(number)).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        eprintln!("Bridge: signal send failed: {e}");
    }
    if let Err(e) = tracker.remove_label(number, LABEL_READY).await {
        eprintln!("Bridge: remove-label ready failed for #{number}: {e}");
    }
    match tracker.add_label(number, LABEL_IN_PROGRESS).await {
        Ok(_) => Ok(format!("#{number} marked as in progress")),
        Err(e) => Err(format!("#{number} signal sent but label failed: {e}")),
    }
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
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "deliver")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let title = tracker
        .get_story_title(number)
        .await
        .unwrap_or_else(|| format!("#{number}"));
    for signal in build_delivery_signals(number, &title) {
        let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
        ws_send(ws_tx, json)
            .await
            .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    }
    if let Err(e) = tracker.remove_label(number, LABEL_IN_PROGRESS).await {
        eprintln!("Bridge: remove-label in-progress failed for #{number}: {e}");
    }
    match tracker.add_label(number, LABEL_DELIVERED).await {
        Ok(_) => Ok(format!("#{number} delivered: signals sent, labels updated")),
        Err(e) => Ok(format!(
            "#{number} delivered: signals sent, label update skipped ({e})"
        )),
    }
}

pub(super) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    tracker: &super::tracker::StoryTracker,
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
    tracker.add_label(number, label).await?;
    Ok(format!("#{number} labeled '{label}'"))
}

pub(super) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["ryan", "pm", "designer", "dev"], "accept")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let role = role_from_name(agent_name);
    let signal = serde_json::to_string(&build_accept_signal(number, role))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        eprintln!("Bridge: signal send failed: {e}");
    }
    if let Err(e) = tracker.remove_label(number, LABEL_DELIVERED).await {
        eprintln!("Bridge: remove-label delivered failed for #{number}: {e}");
    }
    let close_result = tracker.close_story(number).await;
    match close_result {
        Ok(_) => Ok(format!("#{number} accepted and closed")),
        Err(e) if e.contains("already closed") || e.contains("is closed") => {
            Ok(format!("#{number} is already accepted — no-op"))
        }
        Err(e) => Err(format!("#{number} signal sent but close failed: {e}")),
    }
}

pub(super) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["ryan"], "reject")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("rejected");
    let signal = serde_json::to_string(&build_reject_signal(number, reason))
        .map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        eprintln!("Bridge: signal send failed: {e}");
    }
    if let Err(e) = tracker.remove_label(number, LABEL_DELIVERED).await {
        eprintln!("Bridge: remove-label delivered failed for #{number}: {e}");
    }
    if let Err(e) = tracker.add_label(number, LABEL_REJECTED).await {
        eprintln!("Bridge: add-label rejected failed for #{number}: {e}");
    }
    match tracker.comment_on_story(number, reason).await {
        Ok(_) => Ok(format!("#{number} rejected: {reason}")),
        Err(e) => Err(format!("#{number} signal sent but comment failed: {e}")),
    }
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
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["designer"], "pass desk-check")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let signal = serde_json::to_string(&build_pass_desk_check_signal(number))
        .map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, signal)
        .await
        .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    match tracker.add_label(number, LABEL_READY_FOR_ACCEPT).await {
        Ok(_) => Ok(format!(
            "#{number} passed desk-check: signal sent, label applied"
        )),
        Err(e) => Ok(format!(
            "#{number} passed desk-check: signal sent, label update skipped ({e})"
        )),
    }
}

/// Fail desk-check — designer only. Signals dev that the story needs
/// fixes (applies the `needs fixes` label). Same Err-vs-Ok split as
/// pass_desk_check: Err on signal send failure, Ok with gh status.
pub(super) async fn handle_fail_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    tracker: &super::tracker::StoryTracker,
) -> Result<String, String> {
    require_role(agent_name, &["designer"], "fail desk-check")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let signal = serde_json::to_string(&build_fail_desk_check_signal(number, reason))
        .map_err(|e| format!("signal: {e}"))?;
    ws_send(ws_tx, signal)
        .await
        .map_err(|e| format!("#{number} signal send failed: {e}"))?;
    match tracker.add_label(number, LABEL_NEEDS_FIXES).await {
        Ok(_) => Ok(format!(
            "#{number} failed desk-check: signal sent, label applied"
        )),
        Err(e) => Ok(format!(
            "#{number} failed desk-check: signal sent, label update skipped ({e})"
        )),
    }
}

/// #194: PM-controlled priority reordering; persists to priority index.
pub(super) async fn handle_reorder_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
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
    Ok(format!("#{number} moved to position {position} in {state}"))
}

// ── Workflow signal builders (#197) ──────────────────────────

pub(super) fn build_delivery_signals(number: u64, title: &str) -> Vec<ClientMessage> {
    vec![
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered — desk-check needed"),
            to: Some(vec!["designer".to_string()]),
            addressee: Some("designer".to_string()),
            message_type: None,
            topic: None,
        },
        ClientMessage::Send {
            text: format!("#{number} ({title}) delivered. queue update: check ready count"),
            to: Some(vec!["pm".to_string()]),
            addressee: Some("pm".to_string()),
            message_type: None,
            topic: None,
        },
    ]
}

pub(super) fn build_start_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} started. ready queue shrunk — check if restocking needed"),
        to: Some(vec!["pm".to_string()]),
        addressee: Some("pm".to_string()),
        message_type: None,
        topic: None,
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
        topic: None,
    }
}

pub(super) fn build_reject_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} rejected: {reason}. fix and re-deliver"),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
        topic: None,
    }
}

pub(super) fn build_pass_desk_check_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} ready for acceptance. passed designer desk-check."),
        to: Some(vec!["ryan".to_string()]),
        addressee: Some("ryan".to_string()),
        message_type: None,
        topic: None,
    }
}

pub(super) fn build_fail_desk_check_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} failed desk-check: {reason} — needs fixes."),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
        topic: None,
    }
}
