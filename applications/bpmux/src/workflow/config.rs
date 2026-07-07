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
    #[serde(default)]
    pub judges: Vec<String>,
    #[serde(default)]
    pub fail_to: Option<String>,
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

    pub fn state_by_id(&self, id: &str) -> Option<&State> {
        self.state.iter().find(|s| s.id == id)
    }

    pub fn judges_of(&self, state_id: &str) -> &[String] {
        self.state_by_id(state_id)
            .map(|s| s.judges.as_slice())
            .unwrap_or(&[])
    }

    pub fn fail_to_of(&self, state_id: &str) -> Option<&str> {
        self.state_by_id(state_id)
            .and_then(|s| s.fail_to.as_deref())
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
