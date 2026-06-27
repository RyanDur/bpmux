#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use super::events::{BoardEvent, EventMeta, LabelDerivedFields, event_wins};
use super::peer::PeerId;
use super::store::BoardStore;
use crate::product_management::board::adapters::github::{
    acceptor_from_labels, horizon_from_labels, iteration_dev_ok_by_from_labels,
    iteration_membership_from_labels, iteration_review_closers_from_labels,
    parent_epic_from_labels, story_type_from_labels_or_title, tags_from_labels,
};
use crate::product_management::story::{
    Acceptor, FreeComment, Gate, Story, StoryState, StoryType, Tag,
};

fn meta(actor: &str, ts_year: i32, peer: &str) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(ts_year, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named(peer),
    }
}

#[test]
fn store_starts_empty() {
    let store = BoardStore::new();
    assert!(store.is_empty());
    assert_eq!(store.len(), 0);
    assert!(store.all_stories().is_empty());
}

#[test]
fn seed_all_populates_stories() {
    let store = BoardStore::new();
    let s1 = Story::new_minimal(
        1,
        "first".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "owner".to_string(),
    );
    store.seed_all(vec![s1]);
    assert_eq!(store.len(), 1);
    assert_eq!(store.story(1).unwrap().title, "first");
}

#[test]
fn story_created_event_populates_store() {
    let store = BoardStore::new();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 500,
        title: "new story".to_string(),
        body: "body text".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    let story = store.story(500).expect("story exists");
    assert_eq!(story.title, "new story");
    assert_eq!(story.state, StoryState::Ready);
    assert_eq!(story.body.as_deref(), Some("body text"));
    assert_eq!(story.filed_by, "pm");
}

#[test]
fn tag_added_acceptor_label_decodes_into_the_stored_acceptor_field() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 8,
        title: "needs an acceptor".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Delivered,
        story_type: StoryType::Feature,
    });
    assert_eq!(
        store.story(8).unwrap().acceptor(),
        None,
        "a freshly created story carries no acceptor label yet"
    );
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 8,
        label: "awaiting owner".to_string(),
    });
    assert_eq!(
        store.story(8).unwrap().acceptor(),
        Some(Acceptor::Owner),
        "the codec decodes the acceptor label into the stored field on the label-add event"
    );
}

#[test]
fn parent_epic_set_event_decodes_into_the_stored_parent_epic_field() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 12,
        title: "child story".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Icebox,
        story_type: StoryType::Feature,
    });
    assert_eq!(
        store.story(12).unwrap().parent_epic(),
        None,
        "a freshly created story has no parent epic yet"
    );
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 12,
        epic_number: 300,
    });
    assert_eq!(
        store.story(12).unwrap().parent_epic(),
        Some(300),
        "the codec decodes the parent-epic label into the stored field on the ParentEpicSet event"
    );
}

#[test]
fn labels_refreshed_purely_assigns_typed_fields_and_leaves_state_to_state_transitioned() {
    let store = BoardStore::new();
    let story = Story::new_minimal(
        77,
        "membered".to_string(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    store.seed_all(vec![story]);

    store.apply(&BoardEvent::LabelsRefreshed {
        meta: meta("pm", 2026, "local"),
        number: 77,
        fields: LabelDerivedFields {
            raw_labels: vec![
                "parent-epic-#7".to_string(),
                "parent-iteration-#1655.106".to_string(),
                "awaiting dev".to_string(),
            ],
            acceptor: Some(crate::product_management::story::Acceptor::Dev),
            parent_epic: Some(7),
        },
    });

    let s = store.story(77).expect("story exists");
    assert_eq!(parent_epic_from_labels(&s.raw_labels), Some(7));
    assert_eq!(
        iteration_membership_from_labels(&s.raw_labels),
        Some((1655, 106))
    );
    assert_eq!(acceptor_from_labels(&s.raw_labels), Some(Acceptor::Dev));
    assert_eq!(
        s.raw_labels,
        vec![
            "parent-epic-#7".to_string(),
            "parent-iteration-#1655.106".to_string(),
            "awaiting dev".to_string()
        ]
    );
    assert_eq!(
        s.state,
        StoryState::InProgress,
        "LabelsRefreshed is a pure typed assign; workflow state flows via StateTransitioned, not the label echo"
    );
}

#[test]
fn state_transitioned_event_updates_existing_story() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 1,
        new_state: StoryState::InProgress,
    });
    assert_eq!(store.story(1).unwrap().state, StoryState::InProgress);
}

#[test]
fn state_transitioned_event_silently_drops_when_story_missing() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 999,
        new_state: StoryState::InProgress,
    });
    assert!(store.story(999).is_none());
}

#[test]
fn gate_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        number: 1,
        gate: Gate::Pickup,
        payload: Some("scope".to_string()),
    });
    store.apply(&BoardEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        number: 1,
        gate: Gate::Delivered,
        payload: Some("observation".to_string()),
    });
    let story = store.story(1).unwrap();
    let gates = story.gate_comments.as_ref().unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0].gate, Gate::Pickup);
    assert_eq!(gates[1].gate, Gate::Delivered);
}

#[test]
fn free_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::FreeCommentAppended {
        meta: meta("designer", 2026, "local"),
        number: 1,
        body: "design note".to_string(),
    });
    let story = store.story(1).unwrap();
    let free = story.free_comments.as_ref().unwrap();
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].body, "design note");
    assert_eq!(free[0].author, "designer");
}

#[test]
fn body_fetched_event_populates_lazy_body() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        1,
        "t".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    assert_eq!(store.story(1).unwrap().body, None);
    store.apply(&BoardEvent::BodyFetched {
        meta: meta("dev", 2026, "local"),
        number: 1,
        body: "fetched body text".to_string(),
        etag: None,
    });
    assert_eq!(
        store.story(1).unwrap().body.as_deref(),
        Some("fetched body text")
    );
}

#[test]
fn free_comments_fetched_event_replaces_lazy_field() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        1,
        "t".to_string(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    let comments = vec![FreeComment {
        body: "c1".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }];
    store.apply(&BoardEvent::FreeCommentsFetched {
        meta: meta("dev", 2026, "local"),
        number: 1,
        comments: comments.clone(),
    });
    assert_eq!(store.story(1).unwrap().free_comments, Some(comments));
}

#[test]
fn stories_in_state_filters_correctly() {
    let store = BoardStore::new();
    store.seed_all(vec![
        Story::new_minimal(
            1,
            "a".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            2,
            "b".to_string(),
            StoryState::Delivered,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            3,
            "c".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
    ]);
    let ready = store.stories_in_state(StoryState::Ready);
    assert_eq!(ready.len(), 2);
    let mut nums: Vec<u64> = ready.iter().map(|s| s.number).collect();
    nums.sort();
    assert_eq!(nums, vec![1, 3]);
}

#[test]
fn event_wins_later_ts_beats_earlier() {
    let earlier = meta("a", 2025, "local");
    let later = meta("b", 2026, "local");
    assert!(event_wins(&later, &earlier));
    assert!(!event_wins(&earlier, &later));
}

#[test]
fn event_wins_tiebreak_by_peer_id_lexical_when_ts_equal() {
    let a = EventMeta {
        actor: "x".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named("aaa"),
    };
    let b = EventMeta {
        actor: "y".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named("zzz"),
    };
    assert!(event_wins(&b, &a));
    assert!(!event_wins(&a, &b));
}

#[test]
fn peer_id_local_is_the_default_for_mvp() {
    let p = PeerId::local();
    assert_eq!(p.as_str(), "local");
}

#[tokio::test]
async fn subscribe_delivers_applied_events_to_listeners() {
    let store = BoardStore::new();
    let mut rx = store.subscribe();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "sub".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, event);
}

#[tokio::test]
async fn subscribe_fans_out_to_multiple_listeners() {
    let store = BoardStore::new();
    let mut a = store.subscribe();
    let mut b = store.subscribe();
    let event = BoardEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        number: 7,
        new_state: StoryState::Delivered,
    };
    store.seed_all(vec![Story::new_minimal(
        7,
        "t".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    )]);
    store.apply(&event);
    let a_event = a.recv().await.expect("a recv");
    let b_event = b.recv().await.expect("b recv");
    assert_eq!(a_event, event);
    assert_eq!(b_event, event);
}

#[tokio::test]
async fn receiver_count_tracks_live_subscribers() {
    let store = BoardStore::new();
    assert_eq!(store.receiver_count(), 0);
    let _rx = store.subscribe();
    assert_eq!(store.receiver_count(), 1);
    let _rx2 = store.subscribe();
    assert_eq!(store.receiver_count(), 2);
    drop(_rx);
    assert_eq!(store.receiver_count(), 1);
}

#[tokio::test]
async fn apply_succeeds_when_no_subscribers_are_attached() {
    let store = BoardStore::new();
    let event = BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 99,
        title: "no-subs".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    };
    store.apply(&event);
    assert!(store.story(99).is_some());
}

#[test]
fn tag_added_event_appends_to_story_tags() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert_eq!(
        tags_from_labels(&store.story(1).unwrap().raw_labels),
        vec![Tag::Blocked]
    );
}

#[test]
fn tag_added_awaiting_label_rederives_acceptor_then_removal_clears_it_1551() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Delivered,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("designer", 2026, "local"),
        number: 1,
        label: "awaiting team".to_string(),
    });
    assert_eq!(
        acceptor_from_labels(&store.story(1).unwrap().raw_labels),
        Some(Acceptor::Team),
        "the awaiting-label add re-derives the typed acceptor (closes the view refresh-gap)"
    );
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("designer", 2026, "local"),
        number: 1,
        label: "awaiting team".to_string(),
    });
    assert_eq!(
        acceptor_from_labels(&store.story(1).unwrap().raw_labels),
        None,
        "removing the awaiting-label clears the acceptor — honest-neutral, not stale"
    );
}

#[test]
fn tag_added_review_close_label_rederives_closers_then_removal_clears_them_1703() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "Iteration Vela".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Iteration,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("designer", 2026, "local"),
        number: 1,
        label: "iteration-review-closed.designer".to_string(),
    });
    assert_eq!(
        iteration_review_closers_from_labels(&store.story(1).unwrap().raw_labels),
        vec!["designer".to_string()],
        "the review-close label add re-derives the closer-set from the recorded act"
    );
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("designer", 2026, "local"),
        number: 1,
        label: "iteration-review-closed.designer".to_string(),
    });
    assert!(
        iteration_review_closers_from_labels(&store.story(1).unwrap().raw_labels).is_empty(),
        "removing the review-close label clears the closer — derived, never stale"
    );
}

#[test]
fn tag_added_dev_ok_label_rederives_approvers_then_removal_clears_them_1736() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "Iteration Vela".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Iteration,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("dev", 2026, "local"),
        number: 1,
        label: "iteration-dev-ok.dev".to_string(),
    });
    assert_eq!(
        iteration_dev_ok_by_from_labels(&store.story(1).unwrap().raw_labels),
        vec!["dev".to_string()],
        "the dev-OK label add re-derives the approver-set from the recorded act"
    );
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("dev", 2026, "local"),
        number: 1,
        label: "iteration-dev-ok.dev".to_string(),
    });
    assert!(
        iteration_dev_ok_by_from_labels(&store.story(1).unwrap().raw_labels).is_empty(),
        "removing the dev-OK label clears the approver — derived, never stale"
    );
}

#[test]
fn tag_added_horizon_label_rederives_horizon_then_removal_clears_it_1738() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "An outcome".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Icebox,
        story_type: StoryType::Outcome,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "horizon.next".to_string(),
    });
    assert_eq!(
        horizon_from_labels(&store.story(1).unwrap().raw_labels),
        Some(crate::product_management::story::Horizon::Next),
        "the horizon label add re-derives the typed horizon"
    );
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "horizon.next".to_string(),
    });
    assert!(
        horizon_from_labels(&store.story(1).unwrap().raw_labels).is_none(),
        "removing the horizon label clears it — derived, never stale"
    );
}

#[test]
fn outcome_membership_set_event_records_the_mapping_1738() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 710,
        title: "Iteration Vela".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Icebox,
        story_type: StoryType::Iteration,
    });
    store.apply(&BoardEvent::OutcomeMembershipSet {
        meta: meta("pm", 2026, "local"),
        number: 710,
        outcome_number: 700,
        position: 2,
    });
    assert_eq!(
        crate::product_management::board::adapters::github::outcome_membership_from_labels(
            &store.story(710).unwrap().raw_labels
        ),
        Some((700, 2)),
        "the event records which outcome the iteration builds, and at what position"
    );
}

#[test]
fn tag_added_unknown_label_is_dropped() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "high-priority".to_string(),
    });
    assert!(tags_from_labels(&store.story(1).unwrap().raw_labels).is_empty());
}

#[test]
fn tag_added_is_idempotent() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert_eq!(
        tags_from_labels(&store.story(1).unwrap().raw_labels),
        vec![Tag::Blocked]
    );
}

#[test]
fn tag_removed_event_strips_tag() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    assert!(tags_from_labels(&store.story(1).unwrap().raw_labels).is_empty());
}

#[test]
fn tag_removed_unknown_label_is_noop() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "blocked".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "high-priority".to_string(),
    });
    assert_eq!(
        tags_from_labels(&store.story(1).unwrap().raw_labels),
        vec![Tag::Blocked]
    );
}

#[test]
fn tag_added_parent_iteration_label_rederives_iteration_membership_1553() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "parent-iteration-#9301.100".to_string(),
    });
    assert_eq!(
        iteration_membership_from_labels(&store.story(1).unwrap().raw_labels),
        Some((9301, 100))
    );
}

#[test]
fn tag_removed_parent_iteration_label_clears_iteration_membership_1553() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "parent-iteration-#9301.100".to_string(),
    });
    store.apply(&BoardEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "parent-iteration-#9301.100".to_string(),
    });
    assert_eq!(
        iteration_membership_from_labels(&store.story(1).unwrap().raw_labels),
        None
    );
}

#[test]
fn story_created_event_carries_story_type() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 1,
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Bug,
    });
    store.apply(&BoardEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        number: 1,
        label: "bug".to_string(),
    });
    assert_eq!(
        story_type_from_labels_or_title(
            &store.story(1).unwrap().raw_labels,
            &store.story(1).unwrap().title
        ),
        StoryType::Bug
    );
}

#[test]
fn parent_epic_set_event_makes_story_read_its_parent_epic() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 100,
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 100,
        epic_number: 50,
    });
    assert_eq!(
        parent_epic_from_labels(&store.story(100).unwrap().raw_labels),
        Some(50)
    );
}

#[test]
fn parent_epic_unset_event_makes_story_read_no_parent_epic() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        number: 101,
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: StoryState::Ready,
        story_type: StoryType::Feature,
    });
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 101,
        epic_number: 51,
    });
    store.apply(&BoardEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        number: 101,
    });
    assert_eq!(
        parent_epic_from_labels(&store.story(101).unwrap().raw_labels),
        None
    );
}

#[test]
fn parent_epic_events_no_op_on_missing_story() {
    let store = BoardStore::new();
    store.apply(&BoardEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        number: 999,
        epic_number: 888,
    });
    store.apply(&BoardEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        number: 999,
    });
    assert!(store.story(999).is_none());
}
