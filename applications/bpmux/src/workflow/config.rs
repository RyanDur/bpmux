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

    pub fn role_ids_for_purpose(&self, purpose: &str) -> Vec<&str> {
        self.role
            .iter()
            .filter(|r| r.purposes.iter().any(|p| p == purpose))
            .map(|r| r.id.as_str())
            .collect()
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
    story_type: crate::product_management::story::StoryKind,
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
    use crate::product_management::story::StoryKind;
    match story_type {
        StoryKind::Feature => GateClass::OwnerWalk,
        StoryKind::Bug => GateClass::TeamVerifiable,
        StoryKind::Chore => GateClass::DevVerifiable,
    }
}
