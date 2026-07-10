#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{
    Acceptor, StoryKind, gate_class_from_acceptor, gate_class_from_acceptor_and_story_type,
};
use workflow::GateClass;

#[test]
fn acceptor_owner_returns_owner_walk() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(Some(Acceptor::Owner), StoryKind::Feature),
        GateClass::OwnerWalk
    );
}

#[test]
fn acceptor_team_returns_team_verifiable() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(Some(Acceptor::Team), StoryKind::Feature),
        GateClass::TeamVerifiable
    );
}

#[test]
fn acceptor_dev_returns_dev_verifiable() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(Some(Acceptor::Dev), StoryKind::Chore),
        GateClass::DevVerifiable
    );
}

#[test]
fn acceptor_overrides_story_type() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(Some(Acceptor::Dev), StoryKind::Feature),
        GateClass::DevVerifiable
    );
}

#[test]
fn falls_back_to_story_type_feature_owner_walk() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(None, StoryKind::Feature),
        GateClass::OwnerWalk
    );
}

#[test]
fn falls_back_to_story_type_bug_team_verifiable() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(None, StoryKind::Bug),
        GateClass::TeamVerifiable
    );
}

#[test]
fn falls_back_to_story_type_chore_dev_verifiable() {
    assert_eq!(
        gate_class_from_acceptor_and_story_type(None, StoryKind::Chore),
        GateClass::DevVerifiable
    );
}

#[test]
fn acceptor_only_none_defaults_team_verifiable() {
    assert_eq!(gate_class_from_acceptor(None), GateClass::TeamVerifiable);
}

#[test]
fn acceptor_only_owner_returns_owner_walk() {
    assert_eq!(
        gate_class_from_acceptor(Some(Acceptor::Owner)),
        GateClass::OwnerWalk
    );
}
