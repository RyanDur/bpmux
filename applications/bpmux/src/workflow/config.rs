use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub section: Vec<Section>,
    #[serde(default)]
    pub state: Vec<State>,
    #[serde(default)]
    pub transition: Vec<Transition>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum GateClass {
    #[default]
    None,
    TeamVerifiable,
    OwnerWalk,
}

impl WorkflowConfig {
    pub fn transition_by_id(&self, id: &str) -> Option<&Transition> {
        self.transition.iter().find(|t| t.id == id)
    }
}
