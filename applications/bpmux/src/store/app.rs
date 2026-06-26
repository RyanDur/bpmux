use crate::store::kernel::Changed;
use crate::store::roster::{self, RosterAction, RosterState};

#[derive(Default)]
pub struct AppState {
    pub roster: RosterState,
}

pub enum AppAction {
    Roster(RosterAction),
}

impl From<RosterAction> for AppAction {
    fn from(action: RosterAction) -> Self {
        Self::Roster(action)
    }
}

pub fn app_reduce(state: &mut AppState, action: &AppAction) -> Changed {
    match action {
        AppAction::Roster(action) => roster::reduce(&mut state.roster, action),
    }
}
