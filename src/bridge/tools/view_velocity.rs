use tokio::sync::Mutex;

use super::helpers::{WsSink, ws_send};

/// Marker the bridge emits when an agent calls view_velocity (#210).
/// Kept in sync with VIEW_VELOCITY_MARKER in terminal/event.rs.
pub(in crate::bridge) const VIEW_VELOCITY_MARKER: &str = "__BPMUX_VIEW_VELOCITY__";

/// `view_velocity` — fetch the recent message log, parse workflow-
/// signal timestamps, and return velocity metrics as JSON. Counts
/// only business-value stories (vertical / stranded-vertical) per
/// #251; honest-plumbing events are still in the log but don't
/// move the feature-velocity needle.
///
/// Also emits VIEW_VELOCITY_MARKER so Ryan's TUI opens the project
/// panel (#210 — agent-triggered surface). Same two-audience pattern
/// as view_roadmap: the JSON return is for the calling agent, the
/// marker side-effect is for Ryan.
pub(in crate::bridge) async fn handle_view_velocity(
    port: u16,
    tracker: &impl crate::bridge::tracker::StoryTracker,
    ws_tx: &Mutex<Option<WsSink>>,
) -> Result<String, String> {
    // Fetch a wide window so the 14-day compute has enough rows; no
    // `?as=` because workflow signals are broadcasts, not topic-
    // scoped. Limit is generous but bounded to avoid unbounded reads.
    let url = format!("http://127.0.0.1:{port}/api/messages?limit=2000");
    let body = reqwest::get(&url)
        .await
        .map_err(|e| format!("failed to fetch: {e}"))?
        .text()
        .await
        .map_err(|e| format!("failed to read response: {e}"))?;
    let messages: Vec<crate::session::Message> =
        serde_json::from_str(&body).map_err(|e| format!("parse messages: {e}: {body}"))?;

    // Pre-fetch labels for every issue number mentioned in a
    // lifecycle signal so compute can synchronously classify each
    // event as feature vs debt. Fetching once per unique issue
    // keeps the cost bounded to the number of stories in-window.
    let mut unique_numbers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for m in &messages {
        if let Some(hash_idx) = m.text.find('#')
            && let Some((num_str, _)) = m.text[hash_idx + 1..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|i| m.text[hash_idx + 1..].split_at(i))
            && let Ok(n) = num_str.parse::<u64>()
        {
            unique_numbers.insert(n);
        }
    }
    let mut feature_numbers: std::collections::HashSet<u64> = std::collections::HashSet::new();
    for n in unique_numbers {
        let labels = tracker.get_story_labels(n).await;
        let title = tracker.get_story_title(n).await.unwrap_or_default();
        if crate::session::velocity::is_feature_verdict(&labels, &title) {
            feature_numbers.insert(n);
        }
    }

    let metrics = crate::session::velocity::compute(&messages, chrono::Utc::now(), |n| {
        feature_numbers.contains(&n)
    });

    // Surface the view in Ryan's TUI alongside the JSON return.
    // Failure to emit is non-fatal — the JSON data is still useful to
    // the calling agent even if the TUI-side effect couldn't fire.
    let marker_msg = crate::transport::protocol::ClientMessage::Send {
        text: VIEW_VELOCITY_MARKER.to_string(),
        to: None,
        addressee: None,
        message_type: Some("system".to_string()),
        topic: None,
        thread_id: None,
    };
    if let Ok(json) = serde_json::to_string(&marker_msg) {
        let _ = ws_send(ws_tx, json).await;
    }

    serde_json::to_string_pretty(&metrics).map_err(|e| format!("serialize: {e}"))
}
