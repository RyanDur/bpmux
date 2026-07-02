#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{TimeZone, Utc};

use super::events::{EventMeta, LabelDerivedFields, StoryEvent, event_wins};
use super::peer::PeerId;
use super::store::{BoardStore, StoreChange};
use crate::product_management::board::adapters::github::{
    acceptor_from_labels, decode_iteration_from_story, parent_epic_from_labels,
    story_type_from_labels_or_title, tags_from_labels,
};
use crate::product_management::epic::EpicId;
use crate::product_management::story::{
    Acceptor, BoardState, FreeComment, Gate, Stage, Story, StoryId, StoryKind, Tag,
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
        StoryId::new(1),
        "first".to_string(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "owner".to_string(),
    );
    store.seed_all(vec![s1]);
    assert_eq!(store.len(), 1);
    assert_eq!(store.story(StoryId::new(1)).unwrap().title, "first");
}

#[test]
fn story_created_event_populates_store() {
    let store = BoardStore::new();
    let event = StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(500),
        title: "new story".to_string(),
        body: "body text".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    };
    store.apply(&event);
    let story = store.story(StoryId::new(500)).expect("story exists");
    assert_eq!(story.title, "new story");
    assert_eq!(story.state, BoardState::Ready);
    assert_eq!(story.body.as_deref(), Some("body text"));
    assert_eq!(story.filed_by, "pm");
}

#[test]
fn acceptor_assigned_event_sets_the_stored_acceptor_field() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(8),
        title: "needs an acceptor".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Delivered,
        story_type: StoryKind::Feature,
    });
    assert_eq!(
        store.story(StoryId::new(8)).unwrap().acceptor(),
        None,
        "a freshly created story carries no acceptor yet"
    );
    store.apply(&StoryEvent::AcceptorAssigned {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(8),
        acceptor: Acceptor::Owner,
    });
    assert_eq!(
        store.story(StoryId::new(8)).unwrap().acceptor(),
        Some(Acceptor::Owner),
        "the AcceptorAssigned event sets the typed acceptor field directly"
    );
}

#[test]
fn parent_epic_set_event_decodes_into_the_stored_parent_epic_field() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(12),
        title: "child story".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Icebox,
        story_type: StoryKind::Feature,
    });
    assert_eq!(
        store.story(StoryId::new(12)).unwrap().parent_epic(),
        None,
        "a freshly created story has no parent epic yet"
    );
    store.apply(&StoryEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(12),
        epic_id: EpicId::new(300),
    });
    assert_eq!(
        store.story(StoryId::new(12)).unwrap().parent_epic(),
        Some(EpicId::new(300)),
        "the ParentEpicSet event sets the typed parent_epic field directly"
    );
}

#[test]
fn tag_set_event_sets_the_typed_tags_field_directly() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(15),
        title: "blockable".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Delivered,
        story_type: StoryKind::Feature,
    });
    assert!(
        store.story(StoryId::new(15)).unwrap().tags().is_empty(),
        "a freshly created story carries no tags yet"
    );
    store.apply(&StoryEvent::TagSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(15),
        tag: Tag::Blocked,
    });
    assert_eq!(
        store.story(StoryId::new(15)).unwrap().tags(),
        [Tag::Blocked],
        "the TagSet event assigns the typed tag directly, not via a label round-trip"
    );
    store.apply(&StoryEvent::TagSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(15),
        tag: Tag::Blocked,
    });
    assert_eq!(
        store.story(StoryId::new(15)).unwrap().tags(),
        [Tag::Blocked],
        "TagSet is idempotent — re-setting the same tag does not duplicate it"
    );
    store.apply(&StoryEvent::TagUnset {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(15),
        tag: Tag::Blocked,
    });
    assert!(
        store.story(StoryId::new(15)).unwrap().tags().is_empty(),
        "the TagUnset event clears the typed tag directly"
    );
}

#[test]
fn labels_refreshed_assigns_the_ready_order_field_from_adapter_decoded_fields() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(18),
        title: "orderable".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    assert_eq!(
        store.story(StoryId::new(18)).unwrap().ready_order(),
        None,
        "a freshly created story has no ready-order yet"
    );
    store.apply(&StoryEvent::LabelsRefreshed {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(18),
        fields: LabelDerivedFields {
            raw_labels: vec!["ready-order.3".to_string()],
            ready_order: Some(3),
            ..Default::default()
        },
    });
    assert_eq!(
        store.story(StoryId::new(18)).unwrap().ready_order(),
        Some(3),
        "the adapter-decoded ready-order lands in the typed field via the LabelsRefreshed resync; apply_raw_labels no longer re-derives it from raw_labels"
    );
}

#[test]
fn labels_refreshed_purely_assigns_typed_fields_and_leaves_state_to_state_transitioned() {
    let store = BoardStore::new();
    let story = Story::new_minimal(
        StoryId::new(77),
        "membered".to_string(),
        BoardState::InProgress(Stage::Dev),
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    store.seed_all(vec![story]);

    store.apply(&StoryEvent::LabelsRefreshed {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(77),
        fields: LabelDerivedFields {
            raw_labels: vec![
                "parent-epic-#7".to_string(),
                "parent-iteration-#1655.106".to_string(),
                "awaiting dev".to_string(),
            ],
            acceptor: Some(crate::product_management::story::Acceptor::Dev),
            parent_epic: Some(EpicId::new(7)),
            tags: Vec::new(),
            ready_order: None,
        },
    });

    let s = store.story(StoryId::new(77)).expect("story exists");
    assert_eq!(parent_epic_from_labels(&s.raw_labels), Some(7));
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
        BoardState::InProgress(Stage::Dev),
        "LabelsRefreshed is a pure typed assign; workflow state flows via StateTransitioned, not the label echo"
    );
}

#[test]
fn state_transitioned_event_updates_existing_story() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        new_state: BoardState::InProgress(Stage::Dev),
    });
    assert_eq!(
        store.story(StoryId::new(1)).unwrap().state,
        BoardState::InProgress(Stage::Dev)
    );
}

#[test]
fn state_transitioned_event_silently_drops_when_story_missing() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(999),
        new_state: BoardState::InProgress(Stage::Dev),
    });
    assert!(store.story(StoryId::new(999)).is_none());
}

#[test]
fn gate_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        gate: Gate::Pickup,
        payload: Some("scope".to_string()),
    });
    store.apply(&StoryEvent::GateCommentAppended {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        gate: Gate::Delivered,
        payload: Some("observation".to_string()),
    });
    let story = store.story(StoryId::new(1)).unwrap();
    let gates = story.gate_comments.as_ref().unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0].gate, Gate::Pickup);
    assert_eq!(gates[1].gate, Gate::Delivered);
}

#[test]
fn free_comment_appended_event_accumulates() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::FreeCommentAppended {
        meta: meta("designer", 2026, "local"),
        id: StoryId::new(1),
        body: "design note".to_string(),
    });
    let story = store.story(StoryId::new(1)).unwrap();
    let free = story.free_comments.as_ref().unwrap();
    assert_eq!(free.len(), 1);
    assert_eq!(free[0].body, "design note");
    assert_eq!(free[0].author, "designer");
}

#[test]
fn body_fetched_event_populates_lazy_body() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        StoryId::new(1),
        "t".to_string(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    assert_eq!(store.story(StoryId::new(1)).unwrap().body, None);
    store.apply(&StoryEvent::BodyFetched {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        body: "fetched body text".to_string(),
        etag: None,
    });
    assert_eq!(
        store.story(StoryId::new(1)).unwrap().body.as_deref(),
        Some("fetched body text")
    );
}

#[test]
fn free_comments_fetched_event_replaces_lazy_field() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        StoryId::new(1),
        "t".to_string(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )]);
    let comments = vec![FreeComment {
        body: "c1".to_string(),
        author: "pm".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap(),
    }];
    store.apply(&StoryEvent::FreeCommentsFetched {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        comments: comments.clone(),
    });
    assert_eq!(
        store.story(StoryId::new(1)).unwrap().free_comments,
        Some(comments)
    );
}

#[test]
fn stories_in_state_filters_correctly() {
    let store = BoardStore::new();
    store.seed_all(vec![
        Story::new_minimal(
            StoryId::new(1),
            "a".to_string(),
            BoardState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            StoryId::new(2),
            "b".to_string(),
            BoardState::Delivered,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
        Story::new_minimal(
            StoryId::new(3),
            "c".to_string(),
            BoardState::Ready,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        ),
    ]);
    let ready = store.stories_in_state(BoardState::Ready);
    assert_eq!(ready.len(), 2);
    let mut nums: Vec<StoryId> = ready.iter().map(|s| s.id).collect();
    nums.sort();
    assert_eq!(nums, vec![StoryId::new(1), StoryId::new(3)]);
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
    let event = StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "sub".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    };
    store.apply(&event);
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, StoreChange::Event(event));
}

#[tokio::test]
async fn subscribe_fans_out_to_multiple_listeners() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        StoryId::new(7),
        "t".to_string(),
        BoardState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    )]);
    let mut a = store.subscribe();
    let mut b = store.subscribe();
    let event = StoryEvent::StateTransitioned {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(7),
        new_state: BoardState::Delivered,
    };
    store.apply(&event);
    let a_event = a.recv().await.expect("a recv");
    let b_event = b.recv().await.expect("b recv");
    assert_eq!(a_event, StoreChange::Event(event.clone()));
    assert_eq!(b_event, StoreChange::Event(event));
}

#[tokio::test]
async fn seeding_changed_stories_notifies_subscribers_so_the_board_rebroadcasts() {
    let store = BoardStore::new();
    let mut rx = store.subscribe();
    store.seed_all(vec![Story::new_minimal(
        StoryId::new(42),
        "ingested".to_string(),
        BoardState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    )]);
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, StoreChange::Resynced);
}

#[tokio::test]
async fn an_empty_seed_changes_nothing_and_stays_quiet() {
    let store = BoardStore::new();
    let mut rx = store.subscribe();
    store.seed_all(vec![]);
    let event = StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(7),
        title: "next".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    };
    store.apply(&event);
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, StoreChange::Event(event));
}

#[tokio::test]
async fn removing_a_present_story_notifies_but_removing_an_absent_one_stays_quiet() {
    let store = BoardStore::new();
    store.seed_all(vec![Story::new_minimal(
        StoryId::new(5),
        "t".to_string(),
        BoardState::Ready,
        chrono::Utc::now(),
        "pm".to_string(),
    )]);
    let mut rx = store.subscribe();
    assert!(store.remove(StoryId::new(404)).is_none());
    store.remove(StoryId::new(5));
    let received = rx.recv().await.expect("receive");
    assert_eq!(received, StoreChange::Resynced);
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
    let event = StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(99),
        title: "no-subs".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    };
    store.apply(&event);
    assert!(store.story(StoryId::new(99)).is_some());
}

#[test]
fn tag_added_awaiting_label_rederives_acceptor_then_removal_clears_it_1551() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Delivered,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::TagAdded {
        meta: meta("designer", 2026, "local"),
        id: StoryId::new(1),
        label: "awaiting team".to_string(),
    });
    assert_eq!(
        acceptor_from_labels(&store.story(StoryId::new(1)).unwrap().raw_labels),
        Some(Acceptor::Team),
        "the awaiting-label add re-derives the typed acceptor (closes the view refresh-gap)"
    );
    store.apply(&StoryEvent::TagRemoved {
        meta: meta("designer", 2026, "local"),
        id: StoryId::new(1),
        label: "awaiting team".to_string(),
    });
    assert_eq!(
        acceptor_from_labels(&store.story(StoryId::new(1)).unwrap().raw_labels),
        None,
        "removing the awaiting-label clears the acceptor — honest-neutral, not stale"
    );
}

fn iteration_at(store: &BoardStore, id: u64) -> crate::product_management::iteration::Iteration {
    decode_iteration_from_story(
        &store
            .backing_story(StoryId::new(id))
            .expect("iteration issue present in the store"),
    )
}

fn seed_iteration_issue(store: &BoardStore, id: u64) {
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(id),
        title: "Iteration Vela".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::default(),
    });
    store.apply(&StoryEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(id),
        label: "iteration".to_string(),
    });
}

#[test]
fn tag_added_review_close_label_rederives_closers_then_removal_clears_them_1703() {
    let store = BoardStore::new();
    seed_iteration_issue(&store, 1);
    store.apply(&StoryEvent::TagAdded {
        meta: meta("designer", 2026, "local"),
        id: StoryId::new(1),
        label: "iteration-review-closed.designer".to_string(),
    });
    assert_eq!(
        iteration_at(&store, 1).review_closers,
        vec!["designer".to_string()],
        "the review-close label add re-derives the closer-set from the recorded act"
    );
    store.apply(&StoryEvent::TagRemoved {
        meta: meta("designer", 2026, "local"),
        id: StoryId::new(1),
        label: "iteration-review-closed.designer".to_string(),
    });
    assert!(
        iteration_at(&store, 1).review_closers.is_empty(),
        "removing the review-close label clears the closer — derived, never stale"
    );
}

#[test]
fn tag_added_dev_ok_label_rederives_approvers_then_removal_clears_them_1736() {
    let store = BoardStore::new();
    seed_iteration_issue(&store, 1);
    store.apply(&StoryEvent::TagAdded {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        label: "iteration-dev-ok.dev".to_string(),
    });
    assert_eq!(
        iteration_at(&store, 1).dev_ok_by,
        vec!["dev".to_string()],
        "the dev-OK label add re-derives the approver-set from the recorded act"
    );
    store.apply(&StoryEvent::TagRemoved {
        meta: meta("dev", 2026, "local"),
        id: StoryId::new(1),
        label: "iteration-dev-ok.dev".to_string(),
    });
    assert!(
        iteration_at(&store, 1).dev_ok_by.is_empty(),
        "removing the dev-OK label clears the approver — derived, never stale"
    );
}

#[test]
fn tag_added_unknown_label_is_dropped() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        label: "high-priority".to_string(),
    });
    assert!(tags_from_labels(&store.story(StoryId::new(1)).unwrap().raw_labels).is_empty());
}

#[test]
fn generic_label_churn_does_not_clobber_the_typed_tags_field() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::TagSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        tag: Tag::Blocked,
    });
    store.apply(&StoryEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        label: "high-priority".to_string(),
    });
    store.apply(&StoryEvent::TagRemoved {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        label: "high-priority".to_string(),
    });
    assert_eq!(
        store.story(StoryId::new(1)).unwrap().tags(),
        [Tag::Blocked],
        "a generic raw-label add/remove must not re-project (clobber) the typed tags field"
    );
}

#[test]
fn story_created_event_carries_story_type() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        title: "t".to_string(),
        body: "b".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Bug,
    });
    store.apply(&StoryEvent::TagAdded {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(1),
        label: "bug".to_string(),
    });
    assert_eq!(
        story_type_from_labels_or_title(
            &store.story(StoryId::new(1)).unwrap().raw_labels,
            &store.story(StoryId::new(1)).unwrap().title
        ),
        StoryKind::Bug
    );
}

#[test]
fn parent_epic_set_event_makes_story_read_its_parent_epic() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(100),
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(100),
        epic_id: EpicId::new(50),
    });
    assert_eq!(
        store.story(StoryId::new(100)).unwrap().parent_epic(),
        Some(EpicId::new(50))
    );
}

#[test]
fn parent_epic_unset_event_makes_story_read_no_parent_epic() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::StoryCreated {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(101),
        title: "child".to_string(),
        body: "".to_string(),
        initial_state: BoardState::Ready,
        story_type: StoryKind::Feature,
    });
    store.apply(&StoryEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(101),
        epic_id: EpicId::new(51),
    });
    store.apply(&StoryEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(101),
    });
    assert_eq!(store.story(StoryId::new(101)).unwrap().parent_epic(), None);
}

#[test]
fn parent_epic_events_no_op_on_missing_story() {
    let store = BoardStore::new();
    store.apply(&StoryEvent::ParentEpicSet {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(999),
        epic_id: EpicId::new(888),
    });
    store.apply(&StoryEvent::ParentEpicUnset {
        meta: meta("pm", 2026, "local"),
        id: StoryId::new(999),
    });
    assert!(store.story(StoryId::new(999)).is_none());
}
