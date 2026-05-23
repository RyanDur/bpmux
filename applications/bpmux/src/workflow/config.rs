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
    DevVerifiable,
}

impl GateClass {
    pub fn label_suffix(self) -> Option<&'static str> {
        match self {
            GateClass::None => None,
            GateClass::TeamVerifiable => Some("team-verifiable"),
            GateClass::OwnerWalk => Some("owner-walk"),
            GateClass::DevVerifiable => Some("dev-verifiable"),
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
        let lower = l.to_ascii_lowercase();
        if lower == "awaiting team" {
            return GateClass::TeamVerifiable;
        }
        if lower == "awaiting owner" {
            return GateClass::OwnerWalk;
        }
        if lower == "awaiting dev" {
            return GateClass::DevVerifiable;
        }
    }
    for l in labels {
        match l.as_str() {
            "gate:owner-walk" => return GateClass::OwnerWalk,
            "gate:team-verifiable" => return GateClass::TeamVerifiable,
            "gate:dev-verifiable" => return GateClass::DevVerifiable,
            _ => {}
        }
    }
    GateClass::TeamVerifiable
}

pub fn gate_class_from_labels_and_story_type(
    labels: &[String],
    story_type: crate::board::domain::StoryType,
) -> GateClass {
    for l in labels {
        let lower = l.to_ascii_lowercase();
        if lower == "awaiting team" {
            return GateClass::TeamVerifiable;
        }
        if lower == "awaiting owner" {
            return GateClass::OwnerWalk;
        }
        if lower == "awaiting dev" {
            return GateClass::DevVerifiable;
        }
    }
    for l in labels {
        match l.as_str() {
            "gate:owner-walk" => return GateClass::OwnerWalk,
            "gate:team-verifiable" => return GateClass::TeamVerifiable,
            "gate:dev-verifiable" => return GateClass::DevVerifiable,
            _ => {}
        }
    }
    match story_type {
        crate::board::domain::StoryType::Feature
        | crate::board::domain::StoryType::Epic
        | crate::board::domain::StoryType::Iteration => GateClass::OwnerWalk,
        crate::board::domain::StoryType::Bug => GateClass::TeamVerifiable,
        crate::board::domain::StoryType::Chore => GateClass::DevVerifiable,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod gate_class_from_labels_and_story_type_tests_1460 {
    use super::*;
    use crate::board::domain::StoryType;

    fn lbl(s: &str) -> String {
        s.to_string()
    }

    #[test]
    fn label_first_priority_awaiting_owner_returns_owner_walk() {
        let labels = vec![lbl("awaiting owner"), lbl("Feature")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn label_first_priority_awaiting_team_returns_team_verifiable() {
        let labels = vec![lbl("awaiting team"), lbl("Feature")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn case_insensitive_label_match() {
        let labels = vec![lbl("Awaiting Team")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::TeamVerifiable
        );
        let labels = vec![lbl("AWAITING OWNER")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_feature_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_bug_team_verifiable() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
            GateClass::TeamVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_chore_dev_verifiable() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn label_first_priority_awaiting_dev_returns_dev_verifiable() {
        let labels = vec![lbl("awaiting dev"), lbl("Chore")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Chore),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn awaiting_dev_label_overrides_feature_story_type() {
        let labels = vec![lbl("awaiting dev")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn legacy_gate_dev_verifiable_label_honored() {
        let labels = vec![lbl("gate:dev-verifiable")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Feature),
            GateClass::DevVerifiable
        );
    }

    #[test]
    fn falls_back_to_story_type_epic_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Epic),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn falls_back_to_story_type_iteration_owner_walk() {
        let labels: Vec<String> = vec![];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Iteration),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn legacy_gate_label_still_honored_when_no_awaiting_label() {
        let labels = vec![lbl("gate:owner-walk")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
            GateClass::OwnerWalk
        );
    }

    #[test]
    fn awaiting_label_wins_over_legacy_gate_label() {
        let labels = vec![lbl("gate:team-verifiable"), lbl("awaiting owner")];
        assert_eq!(
            gate_class_from_labels_and_story_type(&labels, StoryType::Bug),
            GateClass::OwnerWalk
        );
    }
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
