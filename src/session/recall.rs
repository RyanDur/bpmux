//! Session-start recall: surface an agent's own prior lessons into their
//! next Claude Code session context.
//!
//! This closes the recall-path gap that produced the two-store duplication
//! (Claude Code's auto-memory AND bpmux's `save_lesson`). With this module
//! wired into the SessionStart hook, bpmux's lesson index becomes both the
//! authoritative memory store AND the auto-surfaced recall path.
//!
//! The individuality principle is enforced structurally: this module filters
//! lessons by `author_role`, so a pm session never sees dev's lessons at
//! startup (and vice versa). Each agent reflects through their own lens.

use std::path::Path;

use crate::session::lessons::{self, AgentLesson};

/// Maximum number of lessons to surface in a single auto-recall. Chosen to
/// give meaningful context without bloating the session start — a hard cap
/// also bounds the impact of a runaway lesson index.
pub const MAX_SURFACED_LESSONS: usize = 10;

/// Maximum characters of the lesson body shown per surfaced entry. Lessons
/// are often multi-paragraph and the full text is useful at recall time via
/// `list_lessons`, but a session-start auto-surface needs a scannable
/// summary. Truncated lessons get an ellipsis so the reader knows more
/// exists.
const MAX_LESSON_PREVIEW_CHARS: usize = 180;

/// Return the top-ranked lessons for the given role, sorted by
/// `reinforcement_count` descending (ties broken by recency). Called by
/// `bpmux recall` and by the SessionStart hook.
pub fn surface_for_role(session_dir: &Path, role: &str) -> Vec<AgentLesson> {
    let index = lessons::load_index(session_dir);
    let mut matching: Vec<AgentLesson> = index
        .lessons
        .into_iter()
        .filter(|l| l.author_role == role)
        .collect();

    // Sort so the most load-bearing lessons come first. Primary key is
    // reinforcement count (how many times the team has reaffirmed the
    // lesson), secondary key is last_reinforced timestamp (newer is more
    // active), tertiary is established timestamp (fallback for brand-new
    // lessons with zero reinforcements).
    matching.sort_by(|a, b| {
        b.reinforcement_count
            .cmp(&a.reinforcement_count)
            .then_with(|| {
                let a_time = a.last_reinforced.as_deref().unwrap_or(&a.established);
                let b_time = b.last_reinforced.as_deref().unwrap_or(&b.established);
                b_time.cmp(a_time)
            })
    });

    matching.truncate(MAX_SURFACED_LESSONS);
    matching
}

/// Format surfaced lessons for injection into Claude Code's session start
/// context. The format is intentionally human-readable because Claude reads
/// it directly — no JSON ceremony needed.
pub fn format_for_surface(role: &str, lessons: &[AgentLesson]) -> String {
    if lessons.is_empty() {
        return format!(
            "bpmux recall: no prior lessons for role '{role}'. \
             Save lessons via the bpmux-bridge `save_lesson` tool during work."
        );
    }
    let mut out = String::new();
    out.push_str(&format!(
        "bpmux recall: {} lesson{} from prior sessions for role '{role}'. \
         These are background context, not current conversation.\n\n",
        lessons.len(),
        if lessons.len() == 1 { "" } else { "s" }
    ));
    for (i, lesson) in lessons.iter().enumerate() {
        let preview = truncate_for_preview(&lesson.lesson);
        out.push_str(&format!(
            "{i}. [{}] {preview} — {}\n",
            lesson.topic,
            reinforcement_label(lesson)
        ));
    }
    out.push_str("\nCall list_lessons() or search_log(topic) for full context on any of these.\n");
    out
}

fn truncate_for_preview(text: &str) -> String {
    // Count chars, not bytes, to avoid splitting multibyte characters.
    let char_count = text.chars().count();
    if char_count <= MAX_LESSON_PREVIEW_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_LESSON_PREVIEW_CHARS).collect();
    format!("{truncated}…")
}

/// Build the activity-message payload posted to `/api/messages` when
/// SessionStart recall fires (#173). The payload is a pure value so
/// the shape is testable without a running bpmux server — the actual
/// POST lives in `lib::emit_recall_activity`.
///
/// The `from` field uses `{role}:claude-code` to match the
/// harness-adapter convention on the channel. `message_type` is
/// `activity` so the line lands in the activity region (quiet, the
/// same channel separation used for status pings) rather than the
/// chat stream.
pub fn build_recall_activity_payload(role: &str, count: usize) -> serde_json::Value {
    let text = if count == 0 {
        format!("no prior lessons recalled at session start for role '{role}'")
    } else {
        let plural = if count == 1 { "lesson" } else { "lessons" };
        format!("{count} {plural} recalled at session start for role '{role}'")
    };
    serde_json::json!({
        "from": format!("{role}:claude-code"),
        "text": text,
        "message_type": "activity",
    })
}

fn reinforcement_label(lesson: &AgentLesson) -> String {
    if lesson.reinforcement_count == 0 {
        "new".to_string()
    } else {
        format!("reinforced {}x", lesson.reinforcement_count)
    }
}
