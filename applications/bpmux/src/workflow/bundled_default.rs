use super::config::{GateClass, Role, Section, State, Transition, WorkflowConfig};

pub fn bundled_default() -> WorkflowConfig {
    WorkflowConfig {
        role: vec![
            Role {
                id: "ryan".into(),
                purposes: vec!["owner".into()],
            },
            Role {
                id: "pm".into(),
                purposes: vec!["pm".into()],
            },
            Role {
                id: "designer".into(),
                purposes: vec!["designer".into()],
            },
            Role {
                id: "dev".into(),
                purposes: vec!["dev".into()],
            },
        ],
        section: vec![
            Section {
                id: "pre-work".into(),
            },
            Section {
                id: "in-work".into(),
            },
            Section {
                id: "post-work".into(),
            },
        ],
        state: vec![
            State {
                id: "icebox".into(),
                section: "pre-work".into(),
            },
            State {
                id: "ready".into(),
                section: "pre-work".into(),
            },
            State {
                id: "in-progress".into(),
                section: "in-work".into(),
            },
            State {
                id: "desk-check".into(),
                section: "in-work".into(),
            },
            State {
                id: "delivered".into(),
                section: "in-work".into(),
            },
            State {
                id: "accepted".into(),
                section: "post-work".into(),
            },
            State {
                id: "retired".into(),
                section: "post-work".into(),
            },
        ],
        transition: vec![
            Transition {
                id: "pickup".into(),
                from: "ready".into(),
                to: "in-progress".into(),
                actor_roles: vec!["dev".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "deliver".into(),
                from: "in-progress".into(),
                to: "desk-check".into(),
                actor_roles: vec!["dev".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "pass-desk-check".into(),
                from: "desk-check".into(),
                to: "delivered".into(),
                actor_roles: vec!["designer".into()],
                gate_class: GateClass::TeamVerifiable,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into()],
            },
            Transition {
                id: "pass-desk-check".into(),
                from: "desk-check".into(),
                to: "delivered".into(),
                actor_roles: vec!["designer".into()],
                gate_class: GateClass::OwnerWalk,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into()],
            },
            Transition {
                id: "pass-desk-check".into(),
                from: "desk-check".into(),
                to: "delivered".into(),
                actor_roles: vec!["designer".into()],
                gate_class: GateClass::DevVerifiable,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into()],
            },
            Transition {
                id: "fail-desk-check".into(),
                from: "desk-check".into(),
                to: "in-progress".into(),
                actor_roles: vec!["pm".into(), "designer".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "reject".into(),
                from: "delivered".into(),
                to: "ready".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "accept".into(),
                from: "delivered".into(),
                to: "accepted".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::TeamVerifiable,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into(), "dev".into()],
            },
            Transition {
                id: "accept".into(),
                from: "delivered".into(),
                to: "accepted".into(),
                actor_roles: vec![],
                gate_class: GateClass::OwnerWalk,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "accept".into(),
                from: "delivered".into(),
                to: "accepted".into(),
                actor_roles: vec!["dev".into()],
                gate_class: GateClass::DevVerifiable,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into()],
            },
            Transition {
                id: "close".into(),
                from: "any".into(),
                to: "retired".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "reorder".into(),
                from: "any".into(),
                to: "any".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "move-with-position".into(),
                from: "any".into(),
                to: "any".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "add-to-iteration".into(),
                from: "any".into(),
                to: "any".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
        ],
    }
}
