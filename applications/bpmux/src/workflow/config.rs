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
    /// Backward-compat lookup — returns the first transition with the
    /// given id. Pre-slice-5 callers used this when only one config
    /// row per id existed. Slice 5+ callers should use
    /// [`Self::transition_for`] which also matches on gate_class.
    pub fn transition_by_id(&self, id: &str) -> Option<&Transition> {
        self.transition.iter().find(|t| t.id == id)
    }

    /// Slice 5: lookup by (id, gate_class). When a transition row
    /// has `gate_class = None` the row is treated as a class-agnostic
    /// fallback — it matches any caller's class.
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
