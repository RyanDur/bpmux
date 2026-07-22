use std::sync::Arc;

use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use store::{AppAction, AppState};

pub struct OutcomeBroadcast {
    hub: Arc<hub::Hub>,
    select: store::Select,
}

impl OutcomeBroadcast {
    pub fn new(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self { hub, select }
    }
}

impl Listener<AppState, AppAction> for OutcomeBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.outcomes, &reaction.current.outcomes) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let select = self.select.clone();
        Box::pin(async move {
            super::shared::broadcast_outcomes(&hub, &select);
            super::shared::broadcast_roadmap(&hub, &select);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct IterationBroadcast {
    hub: Arc<hub::Hub>,
    select: store::Select,
}

impl IterationBroadcast {
    pub fn new(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self { hub, select }
    }
}

impl Listener<AppState, AppAction> for IterationBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.iterations, &reaction.current.iterations) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let select = self.select.clone();
        Box::pin(async move {
            super::shared::broadcast_iterations(&hub, &select);
            super::shared::broadcast_outcomes(&hub, &select);
            super::shared::broadcast_roadmap(&hub, &select);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct EpicBroadcast {
    hub: Arc<hub::Hub>,
    select: store::Select,
}

impl EpicBroadcast {
    pub fn new(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self { hub, select }
    }
}

impl Listener<AppState, AppAction> for EpicBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.epics, &reaction.current.epics) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let select = self.select.clone();
        Box::pin(async move {
            super::shared::broadcast_epics(&hub, &select);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}

pub struct StoryBroadcast {
    hub: Arc<hub::Hub>,
    select: store::Select,
}

impl StoryBroadcast {
    pub fn new(hub: Arc<hub::Hub>, select: store::Select) -> Self {
        Self { hub, select }
    }
}

impl Listener<AppState, AppAction> for StoryBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        if !matches!(reaction.action, AppAction::Story(_)) {
            return Box::pin(async {});
        }
        let hub = Arc::clone(&self.hub);
        let select = self.select.clone();
        Box::pin(async move {
            super::shared::broadcast_stories(&hub, &select);
            super::shared::broadcast_epics(&hub, &select);
            super::shared::broadcast_outcomes(&hub, &select);
            super::shared::broadcast_iterations(&hub, &select);
            super::shared::broadcast_roadmap(&hub, &select);
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
