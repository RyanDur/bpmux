use chrono::{TimeZone, Utc};

use super::{apply, created, meta, minimal, seed, seeded, story_at};
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::{FreeComment, StoryId, Verdict, Work};

#[test]
fn verdict_cast_event_accumulates() {
    let store = seeded();
    created(&store, 1, "t", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(1),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Succeed {
                    step: "ready".into(),
                },
                payload: Some("scope".to_string()),
            },
        ),
    );
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(1),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Succeed {
                    step: "in-progress".into(),
                },
                payload: Some("observation".to_string()),
            },
        ),
    );
    let story = story_at(&store, 1).unwrap();
    let gates = story.verdicts.as_ref().unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(
        gates[0].verdict,
        Verdict::Succeed {
            step: "ready".into()
        }
    );
    assert_eq!(
        gates[1].verdict,
        Verdict::Succeed {
            step: "in-progress".into()
        }
    );
}

#[test]
fn free_comment_appended_event_accumulates() {
    let store = seeded();
    created(&store, 1, "t", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("designer", 2026),
            StoryId::new(1),
            StoryEventKind::FreeCommentAppended {
                body: "design note".to_string(),
            },
        ),
    );
    let story = story_at(&store, 1).unwrap();
    let free = story.free_comments.as_ref().unwrap();
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].body, "design note");
    assert_eq!(free[0].author, "designer");
}

#[test]
fn body_fetched_event_populates_lazy_body() {
    let store = seeded();
    seed(&store, vec![minimal(1, "t", Work::NotStarted)]);
    assert_eq!(story_at(&store, 1).unwrap().body, None);
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(1),
            StoryEventKind::BodyFetched {
                body: "fetched body text".to_string(),
                etag: None,
            },
        ),
    );
    assert_eq!(
        story_at(&store, 1).unwrap().body.as_deref(),
        Some("fetched body text")
    );
}

#[test]
fn a_metadata_reseed_preserves_the_store_only_optimistic_verdict() {
    let store = seeded();
    created(&store, 700, "reopened", Work::NotStarted);
    apply(
        &store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(700),
            StoryEventKind::VerdictCast {
                verdict: Verdict::Fail {
                    step: "acceptance".into(),
                },
                payload: Some("accepted in error".to_string()),
            },
        ),
    );
    seed(&store, vec![minimal(700, "reopened", Work::NotStarted)]);
    let gates = story_at(&store, 700)
        .unwrap()
        .verdicts
        .expect("the optimistic reject survives a metadata re-seed — the tracker never round-trips gate history through metadata ingest, so ingest carries verdicts: None and must not clobber the store-only cast");
    assert_eq!(gates.len(), 1, "the cast reject is the sole verdict");
    assert_eq!(
        gates[0].verdict,
        Verdict::Fail {
            step: "acceptance".into()
        }
    );
    assert_eq!(gates[0].payload.as_deref(), Some("accepted in error"));
}

fn locally_transition(store: &store_test_support::SeededStore, id: u64, new_state: Work) {
    apply(
        store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(id),
            StoryEventKind::StateTransitioned {
                new_state,
                placement: None,
            },
        ),
    );
}

#[test]
fn a_reseed_with_absent_remote_freshness_preserves_the_locally_written_state() {
    let store = seeded();
    created(&store, 800, "reopened", Work::NotStarted);
    locally_transition(&store, 800, Work::Accepted);

    seed(&store, vec![minimal(800, "reopened", Work::NotStarted)]);

    assert_eq!(
        story_at(&store, 800).unwrap().state,
        Work::Accepted,
        "an undated reseed carries no freshness and cannot supersede the dated local state write"
    );
}

#[test]
fn a_reseed_with_older_remote_freshness_preserves_the_locally_written_state() {
    let store = seeded();
    created(&store, 801, "reopened", Work::NotStarted);
    locally_transition(&store, 801, Work::Accepted);

    let mut stale = minimal(801, "reopened", Work::NotStarted);
    stale.remote_updated_at = Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
    seed(&store, vec![stale]);

    assert_eq!(
        story_at(&store, 801).unwrap().state,
        Work::Accepted,
        "a remote snapshot older than the local state write is stale and cannot supersede it"
    );
}

#[test]
fn a_reseed_with_newer_remote_freshness_applies_the_remote_state() {
    let store = seeded();
    created(&store, 802, "reopened", Work::NotStarted);
    locally_transition(&store, 802, Work::Accepted);

    let mut fresh = minimal(802, "reopened", Work::InProgress);
    fresh.remote_updated_at = Some(Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap());
    seed(&store, vec![fresh]);

    let after = story_at(&store, 802).unwrap();
    assert_eq!(
        after.state,
        Work::InProgress,
        "a remote snapshot newer than the local state write wins — the genuine-reopen path stays possible"
    );
    assert_eq!(
        after.state_written_at, None,
        "once remote authority applies, the local-write marker clears so subsequent snapshots read remote-authoritative"
    );
}

#[test]
fn free_comments_fetched_event_replaces_lazy_field() {
    let store = seeded();
    seed(&store, vec![minimal(1, "t", Work::NotStarted)]);
    let comments = vec![FreeComment {
        body: "c1".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }];
    apply(
        &store,
        StoryEvent::new(
            meta("dev", 2026),
            StoryId::new(1),
            StoryEventKind::FreeCommentsFetched {
                comments: comments.clone(),
            },
        ),
    );
    assert_eq!(story_at(&store, 1).unwrap().free_comments, Some(comments));
}
