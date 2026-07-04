use std::sync::Arc;

use crate::kernel::{Listener, ListenerFuture, Reaction};
use crate::store::{AppAction, AppState};

pub struct OutcomeBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl OutcomeBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::sync::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for OutcomeBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.outcomes, &reaction.current.outcomes) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_outcomes(&session, &refresh);
            super::shared::broadcast_roadmap(&session, &refresh);
        })
    }
}

pub struct IterationBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl IterationBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::sync::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for IterationBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.iterations, &reaction.current.iterations) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_iterations(&session, &refresh);
            super::shared::broadcast_outcomes(&session, &refresh);
            super::shared::broadcast_roadmap(&session, &refresh);
        })
    }
}

pub struct EpicBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl EpicBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::sync::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for EpicBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.epics, &reaction.current.epics) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_epics(&session, &refresh);
        })
    }
}

pub struct StoryBroadcast {
    session: Arc<crate::session::Session>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl StoryBroadcast {
    pub fn new(
        session: Arc<crate::session::Session>,
        refresh: Arc<dyn crate::sync::RefreshDriver>,
    ) -> Self {
        Self { session, refresh }
    }
}

impl Listener<AppState, AppAction> for StoryBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if !matches!(reaction.action, AppAction::Story(_)) {
            return Box::pin(async {});
        }
        let session = Arc::clone(&self.session);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_stories(&session, &refresh);
            super::shared::broadcast_epics(&session, &refresh);
            super::shared::broadcast_outcomes(&session, &refresh);
            super::shared::broadcast_iterations(&session, &refresh);
            super::shared::broadcast_roadmap(&session, &refresh);
        })
    }
}
