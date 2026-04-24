use crate::bridge::tracker::StoryTracker;

pub(in crate::bridge) const PICKUP_MARKER: &str = "pickup_paraphrase";
pub(in crate::bridge) const DELIVERED_MARKER: &str = "verified_observation";
pub(in crate::bridge) const REJECTION_MARKER: &str = "rejection_reason";
pub(in crate::bridge) const PASS_DESK_CHECK_MARKER: &str = "pass_desk_check";
pub(in crate::bridge) const ACCEPTED_MARKER: &str = "accepted";

pub(in crate::bridge) const ACCEPT_GATE: &str = "accept-gate";
pub(in crate::bridge) const DESK_CHECK_PASS_GATE: &str = "desk-check-pass";
pub(in crate::bridge) const DESK_CHECK_FAIL_GATE: &str = "desk-check-fail";

pub(in crate::bridge) fn format_gate_transition_comment(
    marker: &str,
    gate: &str,
    agent: &str,
) -> String {
    format!("**{marker}** ({gate} by {agent})")
}

pub fn build_timeline(
    story: &serde_json::Value,
    comments: &[serde_json::Value],
) -> Vec<serde_json::Value> {
    let mut events: Vec<serde_json::Value> = Vec::new();

    if let Some(filed) = filed_event(story) {
        events.push(filed);
    }

    for comment in comments {
        if let Some(event) = comment_to_event(comment) {
            events.push(event);
        }
    }

    events.sort_by(|a, b| {
        let ak = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        let bk = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
        ak.cmp(bk)
    });
    events
}

fn filed_event(story: &serde_json::Value) -> Option<serde_json::Value> {
    let ts = story.get("created_at").and_then(|v| v.as_str())?;
    let actor = story
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    Some(serde_json::json!({
        "gate": "filed",
        "ts": ts,
        "actor": actor,
        "payload": serde_json::Value::Null,
    }))
}

fn comment_to_event(comment: &serde_json::Value) -> Option<serde_json::Value> {
    let body = comment.get("body").and_then(|v| v.as_str())?;
    let ts = comment.get("created_at").and_then(|v| v.as_str())?;
    let actor = comment
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let gate = classify_marker(body)?;
    Some(serde_json::json!({
        "gate": gate,
        "ts": ts,
        "actor": actor,
        "payload": body,
    }))
}

fn classify_marker(body: &str) -> Option<&'static str> {
    if body.contains(&wrap_marker(PICKUP_MARKER)) {
        Some("pickup")
    } else if body.contains(&wrap_marker(DELIVERED_MARKER)) {
        Some("delivered")
    } else if body.contains(&wrap_marker(PASS_DESK_CHECK_MARKER)) {
        Some("pass_desk_check")
    } else if body.contains(&wrap_marker(ACCEPTED_MARKER)) {
        Some("accepted")
    } else if body.contains(&wrap_marker(REJECTION_MARKER)) {
        if body.contains(DESK_CHECK_FAIL_GATE) {
            Some("fail_desk_check")
        } else {
            Some("rejected")
        }
    } else {
        None
    }
}

fn wrap_marker(marker: &str) -> String {
    format!("**{marker}**")
}

pub(in crate::bridge) async fn handle_view_story_timeline(
    args: &serde_json::Map<String, serde_json::Value>,
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let story = tracker
        .get_story(number)
        .await?
        .ok_or_else(|| format!("story #{number} not found"))?;
    let comments = tracker.list_story_comments(number).await?;
    let timeline = build_timeline(&story, &comments);
    serde_json::to_string_pretty(&timeline).map_err(|e| format!("serialize timeline: {e}"))
}
