use super::config::{GateClass, Role, Section, State, Transition, WorkflowConfig};

pub fn bundled_default() -> WorkflowConfig {
    WorkflowConfig {
        role: vec![
            Role {
                name: "owner".into(),
                purposes: vec!["owner".into()],
            },
            Role {
                name: "pm".into(),
                purposes: vec!["pm".into()],
            },
            Role {
                name: "designer".into(),
                purposes: vec!["designer".into(), "iteration-reviewer".into()],
            },
            Role {
                name: "dev".into(),
                purposes: vec!["dev".into(), "iteration-builder".into()],
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
                judges: vec![],
                fail_to: None,
            },
            State {
                id: "ready".into(),
                section: "pre-work".into(),
                judges: vec![],
                fail_to: None,
            },
            State {
                id: "in-progress".into(),
                section: "in-work".into(),
                judges: vec![],
                fail_to: None,
            },
            State {
                id: "acceptance".into(),
                section: "post-work".into(),
                judges: vec![],
                fail_to: None,
            },
            State {
                id: "accepted".into(),
                section: "post-work".into(),
                judges: vec![],
                fail_to: None,
            },
            State {
                id: "retired".into(),
                section: "post-work".into(),
                judges: vec![],
                fail_to: None,
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
                to: "acceptance".into(),
                actor_roles: vec!["dev".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "reject".into(),
                from: "acceptance".into(),
                to: "ready".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::None,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "accept".into(),
                from: "acceptance".into(),
                to: "accepted".into(),
                actor_roles: vec!["pm".into()],
                gate_class: GateClass::TeamVerifiable,
                evidence_window_seconds: 600,
                required_evidence_roles: vec!["designer".into(), "dev".into()],
            },
            Transition {
                id: "accept".into(),
                from: "acceptance".into(),
                to: "accepted".into(),
                actor_roles: vec![],
                gate_class: GateClass::OwnerWalk,
                evidence_window_seconds: 0,
                required_evidence_roles: vec![],
            },
            Transition {
                id: "accept".into(),
                from: "acceptance".into(),
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
