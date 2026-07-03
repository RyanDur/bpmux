use crate::workflow::GateClass;

use super::{Acceptor, StoryKind};

fn gate_class_for_acceptor(acceptor: Acceptor) -> GateClass {
    match acceptor {
        Acceptor::Team => GateClass::TeamVerifiable,
        Acceptor::Owner => GateClass::OwnerWalk,
        Acceptor::Dev => GateClass::DevVerifiable,
    }
}

pub fn gate_class_from_acceptor(acceptor: Option<Acceptor>) -> GateClass {
    acceptor
        .map(gate_class_for_acceptor)
        .unwrap_or(GateClass::TeamVerifiable)
}

pub fn gate_class_from_acceptor_and_story_type(
    acceptor: Option<Acceptor>,
    story_type: StoryKind,
) -> GateClass {
    acceptor
        .map(gate_class_for_acceptor)
        .unwrap_or_else(|| match story_type {
            StoryKind::Feature => GateClass::OwnerWalk,
            StoryKind::Bug => GateClass::TeamVerifiable,
            StoryKind::Chore => GateClass::DevVerifiable,
        })
}
