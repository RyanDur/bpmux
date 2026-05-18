use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub section: Vec<Section>,
    #[serde(default)]
    pub state: Vec<State>,
    #[serde(default)]
    pub transition: Vec<Transition>,
    #[serde(default)]
    pub role: Vec<Role>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    #[serde(default)]
    pub purposes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub id: String,
    pub section: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub id: String,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub actor_roles: Vec<String>,
    /// Per-class binding. A transition row matches a fire when both
    /// `id` AND `gate_class` align with the story-axis class label.
    /// Multiple rows can share an `id` provided they differ on
    /// `gate_class` — that's how owner-walk vs team-verifiable accept
    /// configs co-exist under the single `accept` transition identity.
    #[serde(default)]
    pub gate_class: GateClass,
    #[serde(default)]
    pub evidence_window_seconds: u64,
    #[serde(default)]
    pub required_evidence_roles: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum GateClass {
    #[default]
    None,
    TeamVerifiable,
    OwnerWalk,
}

impl GateClass {
    pub fn label_suffix(self) -> Option<&'static str> {
        match self {
            GateClass::None => None,
            GateClass::TeamVerifiable => Some("team-verifiable"),
            GateClass::OwnerWalk => Some("owner-walk"),
        }
    }
}

impl WorkflowConfig {
    pub fn transition_by_id(&self, id: &str) -> Option<&Transition> {
        self.transition.iter().find(|t| t.id == id)
    }

    pub fn transition_for(&self, id: &str, class: GateClass) -> Option<&Transition> {
        self.transition
            .iter()
            .find(|t| t.id == id && t.gate_class == class)
            .or_else(|| {
                self.transition
                    .iter()
                    .find(|t| t.id == id && t.gate_class == GateClass::None)
            })
    }

    pub fn role_id_for_purpose(&self, purpose: &str) -> Option<&str> {
        self.role
            .iter()
            .find(|r| r.purposes.iter().any(|p| p == purpose))
            .map(|r| r.id.as_str())
    }

    pub fn owner_role_id(&self) -> Option<&str> {
        self.role_id_for_purpose("owner")
    }
}

pub fn gate_class_from_labels(labels: &[String]) -> GateClass {
    for l in labels {
        match l.as_str() {
            "gate:owner-walk" => return GateClass::OwnerWalk,
            "gate:team-verifiable" => return GateClass::TeamVerifiable,
            _ => {}
        }
    }
    GateClass::TeamVerifiable
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod role_resolver_tests {
    use super::*;

    fn workflow_with_roles(roles: Vec<Role>) -> WorkflowConfig {
        WorkflowConfig {
            section: vec![],
            state: vec![],
            transition: vec![],
            role: roles,
        }
    }

    #[test]
    fn role_id_for_purpose_returns_matching_role() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("owner"), Some("alice"));
    }

    #[test]
    fn role_id_for_purpose_returns_none_when_no_role_matches() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["owner".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("pm"), None);
    }

    #[test]
    fn role_can_carry_multiple_purposes() {
        let workflow = workflow_with_roles(vec![Role {
            id: "alice".into(),
            purposes: vec!["pm".into(), "lead".into()],
        }]);
        assert_eq!(workflow.role_id_for_purpose("pm"), Some("alice"));
        assert_eq!(workflow.role_id_for_purpose("lead"), Some("alice"));
    }

    #[test]
    fn owner_role_id_resolves_via_owner_purpose() {
        let workflow = workflow_with_roles(vec![
            Role {
                id: "alice".into(),
                purposes: vec!["owner".into()],
            },
            Role {
                id: "bob".into(),
                purposes: vec!["dev".into()],
            },
        ]);
        assert_eq!(workflow.owner_role_id(), Some("alice"));
    }

    #[test]
    fn owner_role_id_returns_none_when_no_owner_purpose() {
        let workflow = workflow_with_roles(vec![Role {
            id: "bob".into(),
            purposes: vec!["dev".into()],
        }]);
        assert_eq!(workflow.owner_role_id(), None);
    }

    #[test]
    fn bundled_default_resolves_canonical_purposes_to_canonical_ids() {
        let workflow = crate::workflow::bundled_default::bundled_default();
        assert_eq!(workflow.role_id_for_purpose("owner"), Some("ryan"));
        assert_eq!(workflow.role_id_for_purpose("pm"), Some("pm"));
        assert_eq!(workflow.role_id_for_purpose("designer"), Some("designer"));
        assert_eq!(workflow.role_id_for_purpose("dev"), Some("dev"));
        assert_eq!(workflow.owner_role_id(), Some("ryan"));
    }
}
