use std::sync::Arc;

use crate::store::{AppAction, AppState};
use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};

pub struct OutcomeBroadcast {
    hub: Arc<crate::hub::Hub>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl OutcomeBroadcast {
    pub fn new(hub: Arc<crate::hub::Hub>, refresh: Arc<dyn crate::sync::RefreshDriver>) -> Self {
        Self { hub, refresh }
    }
}

impl Listener<AppState, AppAction> for OutcomeBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.outcomes, &reaction.current.outcomes) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_outcomes(&hub, &refresh);
            super::shared::broadcast_roadmap(&hub, &refresh);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct IterationBroadcast {
    hub: Arc<crate::hub::Hub>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl IterationBroadcast {
    pub fn new(hub: Arc<crate::hub::Hub>, refresh: Arc<dyn crate::sync::RefreshDriver>) -> Self {
        Self { hub, refresh }
    }
}

impl Listener<AppState, AppAction> for IterationBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.iterations, &reaction.current.iterations) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_iterations(&hub, &refresh);
            super::shared::broadcast_outcomes(&hub, &refresh);
            super::shared::broadcast_roadmap(&hub, &refresh);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct EpicBroadcast {
    hub: Arc<crate::hub::Hub>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl EpicBroadcast {
    pub fn new(hub: Arc<crate::hub::Hub>, refresh: Arc<dyn crate::sync::RefreshDriver>) -> Self {
        Self { hub, refresh }
    }
}

impl Listener<AppState, AppAction> for EpicBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.epics, &reaction.current.epics) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_epics(&hub, &refresh);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct StoryBroadcast {
    hub: Arc<crate::hub::Hub>,
    refresh: Arc<dyn crate::sync::RefreshDriver>,
}

impl StoryBroadcast {
    pub fn new(hub: Arc<crate::hub::Hub>, refresh: Arc<dyn crate::sync::RefreshDriver>) -> Self {
        Self { hub, refresh }
    }
}

impl Listener<AppState, AppAction> for StoryBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if !matches!(reaction.action, AppAction::Story(_)) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let refresh = Arc::clone(&self.refresh);
        Box::pin(async move {
            super::shared::broadcast_stories(&hub, &refresh);
            super::shared::broadcast_epics(&hub, &refresh);
            super::shared::broadcast_outcomes(&hub, &refresh);
            super::shared::broadcast_iterations(&hub, &refresh);
            super::shared::broadcast_roadmap(&hub, &refresh);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
