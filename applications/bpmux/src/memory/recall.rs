use std::path::Path;

use super::lessons::{self, AgentLesson};

pub const MAX_SURFACED_LESSONS: usize = 10;

const MAX_LESSON_PREVIEW_CHARS: usize = 180;

pub fn surface_for_role(session_dir: &Path, role: &str) -> Vec<AgentLesson> {
    let index = lessons::load_index(session_dir);
    let mut matching: Vec<AgentLesson> = index
        .lessons
        .into_iter()
        .filter(|l| l.author_role == role)
        .collect();

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
    let char_count = text.chars().count();
    if char_count <= MAX_LESSON_PREVIEW_CHARS {
        return text.to_string();
    }
    let truncated: String = text.chars().take(MAX_LESSON_PREVIEW_CHARS).collect();
    format!("{truncated}…")
}

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
