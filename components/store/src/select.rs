use std::sync::Weak;

use super::AppAction;
use super::app::AppState;

#[derive(Clone)]
pub struct Select {
    app: Weak<kernel::Store<AppState, AppAction>>,
}

impl Select {
    pub fn new(app: Weak<kernel::Store<AppState, AppAction>>) -> Self {
        Self { app }
    }

    pub fn select<T: Default>(&self, selector: impl FnOnce(&AppState) -> T) -> T {
        match self.app.upgrade() {
            Some(store) => store.select(selector),
            None => T::default(),
        }
    }
}
