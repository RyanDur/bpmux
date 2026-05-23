#![allow(clippy::unwrap_used)]

use tempfile::TempDir;

use super::lessons::{AgentLesson, LessonIndex, save_index};
use super::recall::{
    MAX_SURFACED_LESSONS, build_recall_activity_payload, format_for_surface, surface_for_role,
};

fn lesson(topic: &str, role: &str, established: &str, reinforcements: u32) -> AgentLesson {
    AgentLesson {
        topic: topic.to_string(),
        lesson: format!("lesson for {topic}"),
        author_role: role.to_string(),
        evidence: vec![],
        established: established.to_string(),
        last_reinforced: if reinforcements > 0 {
            Some(established.to_string())
        } else {
            None
        },
        reinforcement_count: reinforcements,
        reinforcement_history: vec![],
    }
}

fn write_index(dir: &TempDir, lessons: Vec<AgentLesson>) {
    let index = LessonIndex {
        experiences: vec![],
        lessons,
    };
    save_index(dir.path(), &index).unwrap();
}

#[test]
fn surfaces_only_matching_role() {
    let dir = TempDir::new().unwrap();
    write_index(
        &dir,
        vec![
            lesson("a", "dev", "2026-04-10T10:00:00Z", 0),
            lesson("b", "pm", "2026-04-10T10:00:00Z", 0),
            lesson("c", "designer", "2026-04-10T10:00:00Z", 0),
            lesson("d", "dev", "2026-04-10T11:00:00Z", 0),
        ],
    );

    let surfaced = surface_for_role(dir.path(), "dev");
    assert_eq!(surfaced.len(), 2);
    assert!(surfaced.iter().all(|l| l.author_role == "dev"));
    let topics: Vec<_> = surfaced.iter().map(|l| l.topic.as_str()).collect();
    assert!(topics.contains(&"a"));
    assert!(topics.contains(&"d"));
}

#[test]
fn pm_never_sees_dev_lessons() {
    let dir = TempDir::new().unwrap();
    write_index(
        &dir,
        vec![
            lesson("dev-lesson", "dev", "2026-04-10T10:00:00Z", 5),
            lesson("pm-lesson", "pm", "2026-04-10T10:00:00Z", 1),
        ],
    );

    let pm_surfaced = surface_for_role(dir.path(), "pm");
    assert_eq!(pm_surfaced.len(), 1);
    assert_eq!(pm_surfaced[0].topic, "pm-lesson");

    let dev_surfaced = surface_for_role(dir.path(), "dev");
    assert_eq!(dev_surfaced.len(), 1);
    assert_eq!(dev_surfaced[0].topic, "dev-lesson");
}

#[test]
fn empty_index_returns_empty_vec() {
    let dir = TempDir::new().unwrap();
    write_index(&dir, vec![]);
    assert!(surface_for_role(dir.path(), "dev").is_empty());
}

#[test]
fn nonexistent_role_returns_empty_vec() {
    let dir = TempDir::new().unwrap();
    write_index(&dir, vec![lesson("a", "dev", "2026-04-10T10:00:00Z", 0)]);
    assert!(surface_for_role(dir.path(), "nonexistent").is_empty());
}

#[test]
fn sorts_by_reinforcement_count_desc() {
    let dir = TempDir::new().unwrap();
    write_index(
        &dir,
        vec![
            lesson("low", "dev", "2026-04-10T10:00:00Z", 1),
            lesson("high", "dev", "2026-04-10T10:00:00Z", 10),
            lesson("mid", "dev", "2026-04-10T10:00:00Z", 5),
        ],
    );

    let surfaced = surface_for_role(dir.path(), "dev");
    assert_eq!(surfaced[0].topic, "high");
    assert_eq!(surfaced[1].topic, "mid");
    assert_eq!(surfaced[2].topic, "low");
}

#[test]
fn ties_broken_by_recency() {
    let dir = TempDir::new().unwrap();
    write_index(
        &dir,
        vec![
            lesson("older", "dev", "2026-04-10T10:00:00Z", 3),
            lesson("newer", "dev", "2026-04-10T11:00:00Z", 3),
            lesson("oldest", "dev", "2026-04-09T23:00:00Z", 3),
        ],
    );

    let surfaced = surface_for_role(dir.path(), "dev");
    assert_eq!(surfaced[0].topic, "newer");
    assert_eq!(surfaced[1].topic, "older");
    assert_eq!(surfaced[2].topic, "oldest");
}

#[test]
fn caps_at_max_surfaced() {
    let dir = TempDir::new().unwrap();
    let lessons: Vec<AgentLesson> = (0..(MAX_SURFACED_LESSONS + 5))
        .map(|i| {
            lesson(
                &format!("lesson-{i}"),
                "dev",
                "2026-04-10T10:00:00Z",
                i as u32,
            )
        })
        .collect();
    write_index(&dir, lessons);

    let surfaced = surface_for_role(dir.path(), "dev");
    assert_eq!(surfaced.len(), MAX_SURFACED_LESSONS);
    assert_eq!(
        surfaced[0].topic,
        format!("lesson-{}", MAX_SURFACED_LESSONS + 4)
    );
}

#[test]
fn format_for_surface_includes_topic_lesson_and_reinforcement() {
    let lessons = vec![lesson("test-topic", "dev", "2026-04-10T10:00:00Z", 3)];
    let out = format_for_surface("dev", &lessons);
    assert!(out.contains("test-topic"));
    assert!(out.contains("lesson for test-topic"));
    assert!(out.contains("reinforced 3x"));
    assert!(out.contains("dev"));
}

#[test]
fn format_for_surface_empty_has_fallback_message() {
    let out = format_for_surface("dev", &[]);
    assert!(out.contains("no prior lessons"));
    assert!(out.contains("dev"));
    assert!(out.contains("save_lesson"));
}

#[test]
fn format_for_surface_marks_new_vs_reinforced() {
    let new_lesson = lesson("fresh", "dev", "2026-04-10T10:00:00Z", 0);
    let out = format_for_surface("dev", &[new_lesson]);
    assert!(out.contains("new"));
    assert!(!out.contains("reinforced 0x"));
}

#[test]
fn activity_payload_reports_lesson_count_and_role() {
    let payload = build_recall_activity_payload("dev", 7);
    assert_eq!(payload["from"], "dev:claude-code");
    assert_eq!(payload["message_type"], "activity");
    let text = payload["text"].as_str().unwrap();
    assert!(
        text.contains("7 lessons"),
        "text should report count: {text}"
    );
    assert!(text.contains("dev"), "text should name the role: {text}");
    assert!(
        text.contains("session start"),
        "text should anchor the event: {text}"
    );
}

#[test]
fn activity_payload_uses_singular_for_one_lesson() {
    let payload = build_recall_activity_payload("pm", 1);
    let text = payload["text"].as_str().unwrap();
    assert!(text.contains("1 lesson "), "singular form: {text}");
    assert!(!text.contains("1 lessons"), "no plural: {text}");
}

#[test]
fn activity_payload_reports_zero_as_no_prior_lessons() {
    let payload = build_recall_activity_payload("designer", 0);
    let text = payload["text"].as_str().unwrap();
    assert!(text.contains("no prior lessons"), "zero phrasing: {text}");
    assert!(text.contains("designer"), "role present: {text}");
}
