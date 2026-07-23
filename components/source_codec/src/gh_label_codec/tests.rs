use super::GhLabelCodec;
use product_management::story::Acceptor;

#[test]
fn decode_acceptor_maps_each_awaiting_label() {
    assert_eq!(
        GhLabelCodec.decode_acceptor("awaiting team"),
        Some(Acceptor::Team)
    );
    assert_eq!(
        GhLabelCodec.decode_acceptor("awaiting owner"),
        Some(Acceptor::Owner)
    );
    assert_eq!(
        GhLabelCodec.decode_acceptor("awaiting dev"),
        Some(Acceptor::Dev)
    );
    assert_eq!(GhLabelCodec.decode_acceptor("feature"), None);
    assert_eq!(GhLabelCodec.decode_acceptor("blocked"), None);
}

#[test]
fn decode_parent_epic_parses_issue_number() {
    assert_eq!(
        GhLabelCodec.decode_parent_epic("parent-epic-#1477"),
        Some(1477)
    );
}

#[test]
fn decode_parent_epic_rejects_non_epic_labels() {
    assert_eq!(GhLabelCodec.decode_parent_epic("iteration-Octans.3"), None);
    assert_eq!(GhLabelCodec.decode_parent_epic("parent-epic-#x"), None);
}

#[test]
fn iteration_review_closed_round_trips_role() {
    let encoded = GhLabelCodec.encode_iteration_review_closed("designer");
    assert_eq!(encoded, "iteration-review-closed.designer");
    assert_eq!(
        GhLabelCodec.decode_iteration_review_closer(&encoded),
        Some("designer".to_string())
    );
}

#[test]
fn decode_iteration_review_closer_rejects_other_labels() {
    assert_eq!(
        GhLabelCodec.decode_iteration_review_closer("iteration-order.3"),
        None
    );
    assert_eq!(
        GhLabelCodec.decode_iteration_review_closer("iteration-review-closed."),
        None
    );
    assert_eq!(GhLabelCodec.decode_iteration_review_closer("feature"), None);
}

#[test]
fn iteration_dev_ok_round_trips_role() {
    let encoded = GhLabelCodec.encode_iteration_dev_ok("dev");
    assert_eq!(encoded, "iteration-dev-ok.dev");
    assert_eq!(
        GhLabelCodec.decode_iteration_dev_ok(&encoded),
        Some("dev".to_string())
    );
}

#[test]
fn decode_iteration_dev_ok_rejects_other_labels() {
    assert_eq!(
        GhLabelCodec.decode_iteration_dev_ok("iteration-order.3"),
        None
    );
    assert_eq!(
        GhLabelCodec.decode_iteration_dev_ok("iteration-dev-ok."),
        None
    );
    assert_eq!(
        GhLabelCodec.decode_iteration_dev_ok("iteration-review-closed.designer"),
        None
    );
}

#[test]
fn horizon_round_trips_each_bucket() {
    use product_management::story::Horizon;
    for (h, label) in [
        (Horizon::Now, "horizon.now"),
        (Horizon::Next, "horizon.next"),
        (Horizon::Later, "horizon.later"),
    ] {
        assert_eq!(GhLabelCodec.encode_horizon(h), label);
        assert_eq!(GhLabelCodec.decode_horizon(label), Some(h));
    }
}

#[test]
fn decode_horizon_rejects_other_labels() {
    assert_eq!(GhLabelCodec.decode_horizon("horizon.someday"), None);
    assert_eq!(GhLabelCodec.decode_horizon("iteration-order.3"), None);
    assert_eq!(GhLabelCodec.decode_horizon("feature"), None);
}

#[test]
fn decode_iteration_order_parses_position() {
    assert_eq!(
        GhLabelCodec.decode_iteration_order("iteration-order.300"),
        Some(300)
    );
    assert_eq!(
        GhLabelCodec.decode_iteration_order("iteration-order.1"),
        Some(1)
    );
}

#[test]
fn decode_iteration_order_rejects_non_order_labels() {
    assert_eq!(
        GhLabelCodec.decode_iteration_order("iteration-Octans.3"),
        None
    );
    assert_eq!(
        GhLabelCodec.decode_iteration_order("iteration-order.x"),
        None
    );
    assert_eq!(GhLabelCodec.decode_iteration_order("ready-order.5"), None);
    assert_eq!(
        GhLabelCodec.decode_iteration_order("parent-iteration-#9.3"),
        None
    );
}

#[test]
fn is_iteration_position_label_accepts_name_dot_digits() {
    assert!(GhLabelCodec.is_iteration_position_label("iteration-Octans.3"));
}

#[test]
fn is_iteration_position_label_excludes_order_and_non_digit_tails() {
    assert!(!GhLabelCodec.is_iteration_position_label("iteration-order.5"));
    assert!(!GhLabelCodec.is_iteration_position_label("iteration-Octans.x"));
    assert!(!GhLabelCodec.is_iteration_position_label("ready-order.5"));
}

#[test]
fn is_workflow_state_label_recognizes_every_canonical_and_legacy_state_label() {
    for label in [
        "icebox",
        "ready",
        "in progress",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "accepted",
    ] {
        assert!(
            GhLabelCodec.is_workflow_state_label(label),
            "{label} is a workflow-state label the transition export must strip when stale"
        );
    }
    assert!(
        GhLabelCodec.is_workflow_state_label("Delivered"),
        "state-label recognition is case-insensitive (GitHub label case can drift)"
    );
}

#[test]
fn is_workflow_state_label_excludes_retired_and_non_state_labels() {
    assert!(
        !GhLabelCodec.is_workflow_state_label("retired"),
        "retired is terminal and must survive transition strips (retire flows add it explicitly)"
    );
    for label in ["Feature", "blocked", "desk-check-fail", "parent-epic-#7"] {
        assert!(
            !GhLabelCodec.is_workflow_state_label(label),
            "{label} is not a workflow-state label and must never be stripped by a transition"
        );
    }
}
