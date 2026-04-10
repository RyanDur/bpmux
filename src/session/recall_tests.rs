#![allow(clippy::unwrap_used)]

//! Tests for session-start recall (#157). The binding constraint is
//! role-scoped isolation — a pm recall must never return dev or designer
//! lessons, and vice versa. Individuality-at-the-implementation-layer.

use tempfile::TempDir;

use super::lessons::{AgentLesson, LessonIndex, save_index};
use super::recall::{MAX_SURFACED_LESSONS, format_for_surface, surface_for_role};

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
    // The load-bearing individuality constraint: cross-role isolation.
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
    // All have the same reinforcement count (3), so recency decides.
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
    // Highest-reinforcement lesson is the first.
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
    // Hint points the agent at save_lesson so they can populate it.
    assert!(out.contains("save_lesson"));
}

#[test]
fn format_for_surface_marks_new_vs_reinforced() {
    let new_lesson = lesson("fresh", "dev", "2026-04-10T10:00:00Z", 0);
    let out = format_for_surface("dev", &[new_lesson]);
    assert!(out.contains("new"));
    assert!(!out.contains("reinforced 0x"));
}
