use super::config::{GateClass, Role, Section, State, Transition, WorkflowConfig};

pub fn bundled_default() -> WorkflowConfig {
    WorkflowConfig {
        role: roles(),
        section: sections(),
        state: states(),
        transition: transitions(),
    }
}

fn role(name: &str, purposes: &[&str]) -> Role {
    Role {
        name: name.into(),
        purposes: purposes.iter().map(|purpose| (*purpose).into()).collect(),
    }
}

fn roles() -> Vec<Role> {
    vec![
        role("owner", &["owner"]),
        role("pm", &["pm"]),
        role("designer", &["designer", "iteration-reviewer"]),
        role("dev", &["dev", "iteration-builder"]),
    ]
}

fn sections() -> Vec<Section> {
    ["pre-work", "in-work", "post-work"]
        .into_iter()
        .map(|id| Section { id: id.into() })
        .collect()
}

fn state(id: &str, section: &str) -> State {
    State {
        id: id.into(),
        section: section.into(),
        judges: vec![],
        fail_to: None,
    }
}

fn states() -> Vec<State> {
    vec![
        state("icebox", "pre-work"),
        state("ready", "pre-work"),
        state("in-progress", "in-work"),
        state("acceptance", "post-work"),
        state("accepted", "post-work"),
        state("retired", "post-work"),
    ]
}

fn ungated(id: &str, from: &str, to: &str, actor_roles: &[&str]) -> Transition {
    Transition {
        id: id.into(),
        from: from.into(),
        to: to.into(),
        actor_roles: actor_roles.iter().map(|actor| (*actor).into()).collect(),
        gate_class: GateClass::None,
        evidence_window_seconds: 0,
        required_evidence_roles: vec![],
    }
}

fn transitions() -> Vec<Transition> {
    vec![
        ungated("pickup", "ready", "in-progress", &["dev"]),
        ungated("deliver", "in-progress", "acceptance", &["dev"]),
        ungated("reject", "acceptance", "ready", &["pm"]),
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
        ungated("close", "any", "retired", &["pm"]),
        ungated("reorder", "any", "any", &["pm"]),
        ungated("move-with-position", "any", "any", &["pm"]),
        ungated("add-to-iteration", "any", "any", &["pm"]),
    ]
}
