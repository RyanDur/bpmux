use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};
use crate::bridge::tracker::StoryTracker;

/// Marker for view_backlog agent-triggered display (#286). Mirrors the
/// #210/#225/#226 pattern — a system-typed broadcast that the TUI
/// intercepts in `event::handle_broadcast_message` to switch view.
pub(in crate::bridge) const VIEW_BACKLOG_MARKER: &str = "__BPMUX_VIEW_BACKLOG__";

/// Sample size for `ready_samples` + `delivered_samples` arrays
/// returned in the JSON payload. Chosen to be enough for an agent
/// to give Ryan a useful "here's what's up" answer without dumping
/// every story.
const SAMPLE_LIMIT: usize = 5;

/// Agent-triggered view_backlog (#286). Emits the system marker so
/// the TUI opens the project panel (Ryan-facing side effect) AND
/// returns a structured JSON summary of the board so the agent can
/// answer "how's the backlog?" in chat. Two audiences, one tool —
/// without the content return, an agent couldn't answer Ryan's
/// question even though the TUI opens.
pub(in crate::bridge) async fn handle_view_backlog(
    ws_tx: &Mutex<Option<WsSink>>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    // Emit the marker first so the TUI opens promptly regardless of
    // how long the tracker fetch takes. Marker-send failure is
    // non-fatal for the agent-callable path — the JSON content is
    // still useful for Ryan's chat-side answer — but we do NOT swallow
    // the error silently. Per fail-loud-for-dev-tools discipline,
    // serialization + send failures are logged to the diag trail so a
    // missing TUI surface is observable from outside the process.
    let marker_msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_BACKLOG_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        thread_id: None,
    };
    match serde_json::to_string(&marker_msg) {
        Ok(json) => {
            if let Err(e) = ws_send(ws_tx, json).await {
                crate::session::diag::log_diag(&format!(
                    "view_backlog marker send failed: {e}"
                ));
            }
        }
        Err(e) => crate::session::diag::log_diag(&format!(
            "view_backlog marker serialize failed: {e}"
        )),
    }

    let json_str = tracker.list_stories().await?;
    let issues: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).map_err(|e| format!("parse: {e}"))?;

    let summary = build_backlog_summary(&issues);
    serde_json::to_string(&summary).map_err(|e| format!("serialize summary: {e}"))
}

/// Pure classifier — partitions the tracker's issue list into the
/// six lifecycle states and emits counts + samples for the two
/// queues dev + Ryan care about most (`ready` and `delivered`).
/// Kept separate from `handle_view_backlog` so it's unit-testable
/// without a live tracker or ws.
fn build_backlog_summary(issues: &[serde_json::Value]) -> serde_json::Value {
    let mut ready: Vec<String> = Vec::new();
    let mut in_progress: Vec<String> = Vec::new();
    let mut delivered: Vec<String> = Vec::new();
    let mut accepted: Vec<String> = Vec::new();
    let mut deferred: Vec<String> = Vec::new();
    let mut icebox: Vec<String> = Vec::new();

    for issue in issues {
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

    let ready_samples: Vec<&String> = ready.iter().take(SAMPLE_LIMIT).collect();
    let delivered_samples: Vec<&String> = delivered.iter().take(SAMPLE_LIMIT).collect();

    serde_json::json!({
        "ready_count": ready.len(),
        "in_progress_count": in_progress.len(),
        "delivered_count": delivered.len(),
        "accepted_count": accepted.len(),
        "deferred_count": deferred.len(),
        "icebox_count": icebox.len(),
        "ready_samples": ready_samples,
        "delivered_samples": delivered_samples,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn issue(num: u64, title: &str, state: &str, labels: &[&str]) -> serde_json::Value {
        let labels_json: Vec<serde_json::Value> = labels
            .iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect();
        serde_json::json!({
            "number": num,
            "title": title,
            "state": state,
            "labels": labels_json,
        })
    }

    #[test]
    fn empty_issues_returns_all_zero_counts() {
        let summary = build_backlog_summary(&[]);
        assert_eq!(summary["ready_count"], 0);
        assert_eq!(summary["in_progress_count"], 0);
        assert_eq!(summary["delivered_count"], 0);
        assert_eq!(summary["accepted_count"], 0);
        assert_eq!(summary["icebox_count"], 0);
        assert_eq!(summary["ready_samples"].as_array().unwrap().len(), 0);
        assert_eq!(summary["delivered_samples"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn closed_issues_classified_as_accepted() {
        let issues = vec![issue(100, "accepted story", "CLOSED", &[])];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["accepted_count"], 1);
        assert_eq!(summary["icebox_count"], 0);
    }

    #[test]
    fn ready_label_classifies_as_ready() {
        let issues = vec![issue(101, "ready story", "OPEN", &["ready"])];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["ready_count"], 1);
        assert_eq!(summary["ready_samples"][0], "#101 ready story");
    }

    #[test]
    fn started_or_finished_labels_classify_as_in_progress() {
        let issues = vec![
            issue(102, "started story", "OPEN", &["started"]),
            issue(103, "finished story", "OPEN", &["finished"]),
        ];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["in_progress_count"], 2);
    }

    #[test]
    fn delivered_label_classifies_as_delivered() {
        let issues = vec![issue(104, "delivered story", "OPEN", &["delivered"])];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["delivered_count"], 1);
        assert_eq!(summary["delivered_samples"][0], "#104 delivered story");
    }

    #[test]
    fn deferred_label_classifies_as_deferred() {
        let issues = vec![issue(105, "deferred story", "OPEN", &["deferred"])];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["deferred_count"], 1);
    }

    #[test]
    fn unlabeled_open_issues_classified_as_icebox() {
        let issues = vec![issue(106, "unlabeled story", "OPEN", &[])];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["icebox_count"], 1);
    }

    #[test]
    fn label_priority_delivered_beats_ready() {
        // A story with both labels should land in delivered, not ready.
        // Mirrors view_board's classification order.
        let issues = vec![issue(
            107,
            "delivered and ready",
            "OPEN",
            &["ready", "delivered"],
        )];
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["delivered_count"], 1);
        assert_eq!(summary["ready_count"], 0);
    }

    #[test]
    fn samples_cap_at_sample_limit() {
        // More ready stories than SAMPLE_LIMIT → only the first N appear.
        let issues: Vec<serde_json::Value> = (0..(SAMPLE_LIMIT as u64 + 3))
            .map(|i| issue(200 + i, &format!("ready {i}"), "OPEN", &["ready"]))
            .collect();
        let summary = build_backlog_summary(&issues);
        assert_eq!(summary["ready_count"], serde_json::json!(SAMPLE_LIMIT + 3));
        assert_eq!(
            summary["ready_samples"].as_array().unwrap().len(),
            SAMPLE_LIMIT
        );
    }
}
