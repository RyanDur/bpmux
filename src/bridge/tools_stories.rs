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

/// Run a gh command and return stdout. Errors include stderr.
/// Sets the working directory to the bpmux repo so `gh` can find
/// git remotes even when the bridge runs from an agent workspace.
pub(super) async fn gh(args: &[&str]) -> Result<String, String> {
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

/// Extract the short role name from the agent's identity.
/// "dev:claude-code" → "dev", "pm" → "pm".
fn role_from_name(name: &str) -> &str {
    name.split(':').next().unwrap_or(name)
}

/// Lane check: returns Ok(()) if allowed, Err with rejection message if not.
fn require_role(agent_name: &str, allowed: &[&str], operation: &str) -> Result<(), String> {
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

/// View board — any agent, no lane restriction.
pub(super) async fn handle_view_board(
    _args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let json_str = gh(&[
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "200",
        "--json",
        "number,title,state,labels",
    ])
    .await?;

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
        } else if has("deferred") {
            deferred.push(entry);
        } else if has("delivered") {
            delivered.push(entry);
        } else if has("started") || has("finished") {
            in_progress.push(entry);
        } else if has("ready") {
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

/// Create a story — any agent can create.
pub(super) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");

    let mut cmd_args = vec!["issue", "create", "--title", title];
    if !body.is_empty() {
        cmd_args.extend(["--body", body]);
    }
    let result = gh(&cmd_args).await?;
    Ok(format!("story created: {}", result.trim()))
}

/// Start a story — dev only. Sends workflow signals (#197).
/// Signals fire on INTENT — team is notified even if gh fails.
pub(super) async fn handle_start_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
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
    let gh_result = gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "ready",
        "--add-label",
        "in progress",
    ])
    .await;
    match gh_result {
        Ok(_) => Ok(format!("#{number} marked as in progress")),
        Err(e) => Err(format!("#{number} signal sent but gh failed: {e}")),
    }
}

/// Deliver a story — dev only. Sends workflow signals (#197).
/// Signals fire on INTENT — team is notified even if gh fails.
pub(super) async fn handle_deliver_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(agent_name, &["dev"], "deliver")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let title = gh(&["issue", "view", &number.to_string(), "--json", "title"])
        .await
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|v| v["title"].as_str().map(String::from))
        .unwrap_or_else(|| format!("#{number}"));
    for signal in build_delivery_signals(number, &title) {
        let json = serde_json::to_string(&signal).map_err(|e| format!("signal: {e}"))?;
        if let Err(e) = ws_send(ws_tx, json).await {
            eprintln!("Bridge: signal send failed: {e}");
        }
    }
    let gh_result = gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "in progress",
        "--add-label",
        "delivered",
    ])
    .await;
    match gh_result {
        Ok(_) => Ok(format!("#{number} marked as delivered")),
        Err(e) => Err(format!("#{number} signal sent but gh failed: {e}")),
    }
}

/// Add/change labels — PM only.
pub(super) async fn handle_label_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
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
    gh(&["issue", "edit", &number.to_string(), "--add-label", label]).await?;
    Ok(format!("#{number} labeled '{label}'"))
}

/// Accept a story — owner (ryan) only. Sends workflow signals (#197).
pub(super) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    require_role(agent_name, &["ryan"], "accept")?;
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let signal =
        serde_json::to_string(&build_accept_signal(number)).map_err(|e| format!("signal: {e}"))?;
    if let Err(e) = ws_send(ws_tx, signal).await {
        eprintln!("Bridge: signal send failed: {e}");
    }
    let _ = gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "delivered",
    ])
    .await;
    let gh_result = gh(&["issue", "close", &number.to_string()]).await;
    match gh_result {
        Ok(_) => Ok(format!("#{number} accepted and closed")),
        Err(e) => Err(format!("#{number} signal sent but gh failed: {e}")),
    }
}

/// Reject a story — owner (ryan) only. Sends workflow signals (#197).
pub(super) async fn handle_reject_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
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
    let _ = gh(&[
        "issue",
        "edit",
        &number.to_string(),
        "--remove-label",
        "delivered",
        "--add-label",
        "rejected",
    ])
    .await;
    let gh_result = gh(&["issue", "comment", &number.to_string(), "--body", reason]).await;
    match gh_result {
        Ok(_) => Ok(format!("#{number} rejected: {reason}")),
        Err(e) => Err(format!("#{number} signal sent but gh failed: {e}")),
    }
}

/// Reorder a story within its state column — PM only (#194).
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

fn build_delivery_signals(number: u64, title: &str) -> Vec<ClientMessage> {
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

fn build_start_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} started. ready queue shrunk — check if restocking needed"),
        to: Some(vec!["pm".to_string()]),
        addressee: Some("pm".to_string()),
        message_type: None,
        topic: None,
    }
}

fn build_accept_signal(number: u64) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} accepted. pick the next story from the ready queue"),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
        topic: None,
    }
}

fn build_reject_signal(number: u64, reason: &str) -> ClientMessage {
    ClientMessage::Send {
        text: format!("#{number} rejected: {reason}. fix and re-deliver"),
        to: Some(vec!["dev".to_string()]),
        addressee: Some("dev".to_string()),
        message_type: None,
        topic: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_from_name_extracts_short_role() {
        assert_eq!(role_from_name("dev:claude-code"), "dev");
        assert_eq!(role_from_name("pm"), "pm");
        assert_eq!(role_from_name("designer:cursor"), "designer");
        assert_eq!(role_from_name("ryan:human"), "ryan");
    }

    #[test]
    fn require_role_allows_matching_role() {
        assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
        assert!(require_role("pm", &["pm"], "label").is_ok());
        assert!(require_role("ryan:human", &["ryan"], "accept").is_ok());
    }

    #[test]
    fn require_role_rejects_non_matching_role() {
        let result = require_role("designer:claude-code", &["dev"], "start");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to dev"));
    }

    #[test]
    fn require_role_rejects_pm_from_dev_operations() {
        let result = require_role("pm:claude-code", &["dev"], "deliver");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to dev"));
    }

    #[test]
    fn require_role_rejects_dev_from_pm_operations() {
        let result = require_role("dev:claude-code", &["pm"], "label");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to pm"));
    }

    #[test]
    fn require_role_rejects_agents_from_owner_operations() {
        let result = require_role("dev:claude-code", &["ryan"], "accept");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to ryan"));

        let result = require_role("pm:claude-code", &["ryan"], "reject");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("restricted to ryan"));
    }

    // ── Workflow signal tests (#197) ──

    #[test]
    fn delivery_signals_target_designer_and_pm() {
        let signals = build_delivery_signals(191, "topic channels");
        assert_eq!(signals.len(), 2, "deliver should produce 2 signals");

        if let ClientMessage::Send { to, addressee, .. } = &signals[0] {
            assert_eq!(to.as_ref().unwrap(), &["designer"]);
            assert_eq!(addressee.as_deref(), Some("designer"));
        } else {
            panic!("expected Send variant");
        }

        if let ClientMessage::Send { to, addressee, .. } = &signals[1] {
            assert_eq!(to.as_ref().unwrap(), &["pm"]);
            assert_eq!(addressee.as_deref(), Some("pm"));
        } else {
            panic!("expected Send variant");
        }
    }

    #[test]
    fn delivery_signals_do_not_target_dev() {
        let signals = build_delivery_signals(191, "topic channels");
        for signal in &signals {
            if let ClientMessage::Send { to, .. } = signal {
                let recipients = to.as_ref().unwrap();
                assert!(
                    !recipients.contains(&"dev".to_string()),
                    "delivery signal should NOT target dev, got: {recipients:?}"
                );
            }
        }
    }

    #[test]
    fn start_signal_targets_pm_only() {
        if let ClientMessage::Send { to, addressee, .. } = build_start_signal(100) {
            assert_eq!(to.as_ref().unwrap(), &["pm"]);
            assert_eq!(addressee.as_deref(), Some("pm"));
        } else {
            panic!("expected Send variant");
        }
    }

    #[test]
    fn accept_signal_targets_dev_only() {
        if let ClientMessage::Send { to, addressee, .. } = build_accept_signal(100) {
            assert_eq!(to.as_ref().unwrap(), &["dev"]);
            assert_eq!(addressee.as_deref(), Some("dev"));
        } else {
            panic!("expected Send variant");
        }
    }

    #[test]
    fn reject_signal_targets_dev_only() {
        if let ClientMessage::Send { to, addressee, .. } = build_reject_signal(100, "needs fix") {
            assert_eq!(to.as_ref().unwrap(), &["dev"]);
            assert_eq!(addressee.as_deref(), Some("dev"));
        } else {
            panic!("expected Send variant");
        }
    }

    #[test]
    fn reject_signal_includes_reason() {
        if let ClientMessage::Send { text, .. } = build_reject_signal(42, "missing AC") {
            assert!(
                text.contains("missing AC"),
                "reason should be in text: {text}"
            );
            assert!(
                text.contains("#42"),
                "story number should be in text: {text}"
            );
        }
    }
}
