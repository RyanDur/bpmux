use super::config::{Section, State, WorkflowConfig};

pub fn bundled_default() -> WorkflowConfig {
    WorkflowConfig {
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
    }
}
