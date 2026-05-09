use serde::Serialize;
use std::sync::Mutex;

pub const CONSTELLATION_THEME_POOL: &[&str] = &[
    "Andromeda",
    "Aquila",
    "Aries",
    "Auriga",
    "Boötes",
    "Cancer",
    "Canis Major",
    "Capricornus",
    "Cassiopeia",
    "Centaurus",
    "Cepheus",
    "Cetus",
    "Columba",
    "Corvus",
    "Crux",
    "Cygnus",
    "Delphinus",
    "Draco",
    "Eridanus",
    "Gemini",
    "Hercules",
    "Hydra",
    "Indus",
    "Leo",
    "Lepus",
    "Libra",
    "Lupus",
    "Lynx",
    "Lyra",
    "Monoceros",
    "Ophiuchus",
    "Orion",
    "Pavo",
    "Pegasus",
    "Perseus",
    "Phoenix",
    "Pisces",
    "Sagittarius",
    "Scorpius",
    "Serpens",
    "Taurus",
    "Triangulum",
    "Tucana",
    "Ursa Major",
    "Ursa Minor",
    "Vela",
    "Virgo",
    "Vulpecula",
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct IterationMarker {
    pub id: u64,
    pub theme_name: String,
    pub position: u32,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IterationMarkersAction {
    MarkerAdded(IterationMarker),
    MarkerRemoved { id: u64 },
}

#[derive(Default)]
pub struct IterationMarkersStore {
    inner: Mutex<IterationMarkersState>,
}

#[derive(Default, Clone)]
struct IterationMarkersState {
    markers: Vec<IterationMarker>,
    next_id: u64,
}

impl IterationMarkersStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Vec<IterationMarker> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut out = state.markers.clone();
        out.sort_by_key(|m| m.position);
        out
    }

    pub fn allocate_id(&self) -> u64 {
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let id = state.next_id;
        state.next_id += 1;
        id
    }

    pub fn next_position(&self) -> u32 {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state
            .markers
            .iter()
            .map(|m| m.position)
            .max()
            .map(|p| p + 1)
            .unwrap_or(0)
    }

    pub fn next_theme_name(&self) -> String {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let used: std::collections::HashSet<&String> =
            state.markers.iter().map(|m| &m.theme_name).collect();
        for theme in CONSTELLATION_THEME_POOL {
            if !used.contains(&theme.to_string()) {
                return theme.to_string();
            }
        }
        format!("Marker {}", state.markers.len() + 1)
    }

    pub fn apply(&self, action: &IterationMarkersAction) -> super::stores::StoreReduceOutcome {
        use super::stores::StoreReduceOutcome;
        let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match action {
            IterationMarkersAction::MarkerAdded(marker) => {
                if state.markers.iter().any(|m| m.id == marker.id) {
                    return StoreReduceOutcome::Unchanged;
                }
                state.markers.push(marker.clone());
                StoreReduceOutcome::Changed
            }
            IterationMarkersAction::MarkerRemoved { id } => {
                let before = state.markers.len();
                state.markers.retain(|m| &m.id != id);
                if state.markers.len() == before {
                    StoreReduceOutcome::Unchanged
                } else {
                    StoreReduceOutcome::Changed
                }
            }
        }
    }
}

pub trait IterationMarkersBroadcastListener: Send + Sync {
    fn on_changed(&self, markers: &[IterationMarker]);
}
