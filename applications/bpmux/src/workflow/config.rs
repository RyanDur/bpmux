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

fn gate_class_for_acceptor(acceptor: crate::product_management::story::Acceptor) -> GateClass {
    use crate::product_management::story::Acceptor;
    match acceptor {
        Acceptor::Team => GateClass::TeamVerifiable,
        Acceptor::Owner => GateClass::OwnerWalk,
        Acceptor::Dev => GateClass::DevVerifiable,
    }
}

pub fn gate_class_from_acceptor(
    acceptor: Option<crate::product_management::story::Acceptor>,
) -> GateClass {
    acceptor
        .map(gate_class_for_acceptor)
        .unwrap_or(GateClass::TeamVerifiable)
}

pub fn gate_class_from_acceptor_and_story_type(
    acceptor: Option<crate::product_management::story::Acceptor>,
    story_type: crate::product_management::story::StoryKind,
) -> GateClass {
    use crate::product_management::story::StoryKind;
    acceptor
        .map(gate_class_for_acceptor)
        .unwrap_or_else(|| match story_type {
            StoryKind::Feature => GateClass::OwnerWalk,
            StoryKind::Bug => GateClass::TeamVerifiable,
            StoryKind::Chore => GateClass::DevVerifiable,
        })
}
