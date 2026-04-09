#![allow(clippy::unwrap_used)]

use super::classifier::{ExperienceKind, MessageClass};
use super::lessons::*;
use super::message::{Message, MessageType};
use chrono::Utc;
use tempfile::TempDir;

fn msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    }
}

#[test]
fn build_index_finds_experiences() {
    let messages = vec![
        msg("ryan:human", "you should restate before building"),
        msg("dev:claude-code", "understood"),
        msg("ryan:human", "sounds good"),
    ];
    let index = build_index(&messages, "ryan");
    assert_eq!(index.experiences.len(), 1);
    assert!(
        index.experiences[0]
            .text_preview
            .contains("restate before building")
    );
}

#[test]
fn build_index_skips_operational() {
    let messages = vec![
        msg("ryan:human", "do I need to restart?"),
        msg("ryan:human", "rebuilt now checking"),
        msg("ryan:human", "sounds good, keep going"),
    ];
    let index = build_index(&messages, "ryan");
    assert_eq!(
        index.experiences.len(),
        0,
        "Operational messages should not be indexed"
    );
}

#[test]
fn group_by_topic_merges_similar() {
    let messages = vec![
        msg("ryan:human", "you should focus on the problem"),
        msg("ryan:human", "focus on what matters, not the fix"),
    ];
    let index = build_index(&messages, "ryan");
    let groups = group_by_topic(&index);
    assert!(
        groups.len() <= 2,
        "Related experiences should group: {groups:?}"
    );
}

#[test]
fn save_and_load_index_roundtrips() {
    let dir = TempDir::new().unwrap();
    let mut index = LessonIndex::default();
    index.experiences.push(ExperienceEntry {
        timestamp: "2026-04-08T00:00:00Z".to_string(),
        keywords: vec!["quality".to_string()],
        evidence: vec!["ts1".to_string()],
        text_preview: "quality is the goal".to_string(),
    });
    save_index(dir.path(), &index).unwrap();
    let loaded = load_index(dir.path());
    assert_eq!(loaded.experiences.len(), 1);
    assert_eq!(loaded.experiences[0].text_preview, "quality is the goal");
}

#[test]
fn add_lesson_creates_entry() {
    let dir = TempDir::new().unwrap();
    add_lesson(dir.path(), "quality", "quality compounds", "dev", vec![]).unwrap();
    let index = load_index(dir.path());
    assert_eq!(index.lessons.len(), 1);
    assert_eq!(index.lessons[0].topic, "quality");
    assert_eq!(index.lessons[0].author_role, "dev");
}

#[test]
fn add_lesson_upserts_same_role_topic() {
    let dir = TempDir::new().unwrap();
    add_lesson(dir.path(), "quality", "first version", "dev", vec![]).unwrap();
    add_lesson(dir.path(), "quality", "updated version", "dev", vec![]).unwrap();
    let index = load_index(dir.path());
    assert_eq!(index.lessons.len(), 1, "Should upsert, not duplicate");
    assert_eq!(index.lessons[0].lesson, "updated version");
}

#[test]
fn add_lesson_keeps_different_roles_separate() {
    let dir = TempDir::new().unwrap();
    add_lesson(dir.path(), "quality", "dev perspective", "dev", vec![]).unwrap();
    add_lesson(dir.path(), "quality", "pm perspective", "pm", vec![]).unwrap();
    let index = load_index(dir.path());
    assert_eq!(
        index.lessons.len(),
        2,
        "Different roles have separate lessons"
    );
}

#[test]
fn group_by_topic_includes_lessons() {
    let dir = TempDir::new().unwrap();
    add_lesson(dir.path(), "quality", "quality compounds", "dev", vec![]).unwrap();
    let mut index = build_index(
        &[msg("ryan:human", "quality is the goal. in everything")],
        "ryan",
    );
    let saved = load_index(dir.path());
    index.lessons = saved.lessons;
    let groups = group_by_topic(&index);
    let quality_group = groups.iter().find(|g| g.topic == "quality");
    assert!(quality_group.is_some(), "Should have a quality topic group");
    assert_eq!(quality_group.unwrap().lessons.len(), 1);
}

#[test]
fn empty_log_produces_empty_index() {
    let index = build_index(&[], "ryan");
    assert!(index.experiences.is_empty());
    assert!(index.lessons.is_empty());
}

#[test]
fn classifier_tags_search_result() {
    use super::search::{SearchFilters, search};
    let messages = vec![
        msg("ryan:human", "you should focus on the problem not the fix"),
        msg("dev:claude-code", "understood, focusing on the problem"),
    ];
    let filters = SearchFilters {
        query: "focus".to_string(),
        sender: None,
        after: None,
        before: None,
        channel: None,
        message_class: Some(MessageClass::Experience(ExperienceKind::Teaching)),
        owner_name: "ryan".to_string(),
    };
    let results = search(&messages, &filters);
    assert_eq!(results.len(), 1);
    assert!(results[0].message_class.is_experience());
}

#[test]
fn merge_adds_new_lessons_from_remote() {
    let mut local = LessonIndex::default();
    local.lessons.push(AgentLesson {
        topic: "quality".to_string(),
        lesson: "quality compounds".to_string(),
        author_role: "dev".to_string(),
        evidence: vec![],
        established: "2026-04-08T00:00:00Z".to_string(),
        last_reinforced: None,
        reinforcement_count: 0,
    });

    let remote = LessonIndex {
        experiences: vec![],
        lessons: vec![AgentLesson {
            topic: "role boundaries".to_string(),
            lesson: "features go through PM".to_string(),
            author_role: "designer".to_string(),
            evidence: vec![],
            established: "2026-04-08T01:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
        }],
    };

    merge_indexes(&mut local, &remote);
    assert_eq!(local.lessons.len(), 2);
}

#[test]
fn merge_keeps_newer_lesson_for_same_topic_and_role() {
    let mut local = LessonIndex::default();
    local.lessons.push(AgentLesson {
        topic: "quality".to_string(),
        lesson: "old version".to_string(),
        author_role: "dev".to_string(),
        evidence: vec![],
        established: "2026-04-08T00:00:00Z".to_string(),
        last_reinforced: None,
        reinforcement_count: 0,
    });

    let remote = LessonIndex {
        experiences: vec![],
        lessons: vec![AgentLesson {
            topic: "quality".to_string(),
            lesson: "newer version".to_string(),
            author_role: "dev".to_string(),
            evidence: vec![],
            established: "2026-04-09T00:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
        }],
    };

    merge_indexes(&mut local, &remote);
    assert_eq!(local.lessons.len(), 1);
    assert_eq!(local.lessons[0].lesson, "newer version");
}

#[test]
fn merge_keeps_local_when_local_is_newer() {
    let mut local = LessonIndex::default();
    local.lessons.push(AgentLesson {
        topic: "quality".to_string(),
        lesson: "local newer".to_string(),
        author_role: "dev".to_string(),
        evidence: vec![],
        established: "2026-04-09T00:00:00Z".to_string(),
        last_reinforced: None,
        reinforcement_count: 0,
    });

    let remote = LessonIndex {
        experiences: vec![],
        lessons: vec![AgentLesson {
            topic: "quality".to_string(),
            lesson: "remote older".to_string(),
            author_role: "dev".to_string(),
            evidence: vec![],
            established: "2026-04-08T00:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
        }],
    };

    merge_indexes(&mut local, &remote);
    assert_eq!(local.lessons.len(), 1);
    assert_eq!(local.lessons[0].lesson, "local newer");
}

#[test]
fn merge_deduplicates_experiences_by_timestamp() {
    let mut local = LessonIndex::default();
    local.experiences.push(ExperienceEntry {
        timestamp: "2026-04-08T00:00:00Z".to_string(),
        keywords: vec!["quality".to_string()],
        evidence: vec![],
        text_preview: "quality is the goal".to_string(),
    });

    let remote = LessonIndex {
        experiences: vec![
            ExperienceEntry {
                timestamp: "2026-04-08T00:00:00Z".to_string(),
                keywords: vec!["quality".to_string()],
                evidence: vec![],
                text_preview: "quality is the goal".to_string(),
            },
            ExperienceEntry {
                timestamp: "2026-04-09T00:00:00Z".to_string(),
                keywords: vec!["focus".to_string()],
                evidence: vec![],
                text_preview: "focus on the problem".to_string(),
            },
        ],
        lessons: vec![],
    };

    merge_indexes(&mut local, &remote);
    assert_eq!(
        local.experiences.len(),
        2,
        "duplicate skipped, new one added"
    );
}

#[test]
fn merge_same_topic_different_roles_both_kept() {
    let mut local = LessonIndex::default();
    local.lessons.push(AgentLesson {
        topic: "quality".to_string(),
        lesson: "dev perspective".to_string(),
        author_role: "dev".to_string(),
        evidence: vec![],
        established: "2026-04-08T00:00:00Z".to_string(),
        last_reinforced: None,
        reinforcement_count: 0,
    });

    let remote = LessonIndex {
        experiences: vec![],
        lessons: vec![AgentLesson {
            topic: "quality".to_string(),
            lesson: "pm perspective".to_string(),
            author_role: "pm".to_string(),
            evidence: vec![],
            established: "2026-04-08T00:00:00Z".to_string(),
            last_reinforced: None,
            reinforcement_count: 0,
        }],
    };

    merge_indexes(&mut local, &remote);
    assert_eq!(local.lessons.len(), 2, "different roles are separate");
}
