use std::sync::Arc;

use product_management::data_gateway::TransitionsDataGateway;
use product_management::story::slice;

pub struct TransitionPersister {
    transitions: Arc<TransitionsDataGateway>,
}

impl TransitionPersister {
    pub fn new(transitions: Arc<TransitionsDataGateway>) -> Self {
        Self { transitions }
    }
}

impl kernel::Listener<crate::AppState, crate::AppAction> for TransitionPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::AppState, crate::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.stories, &reaction.current.stories) {
            return Box::pin(async {});
        }
        let observed_at = chrono::Utc::now().timestamp();
        let mut changed = Vec::new();
        for story in slice::all(&reaction.current.stories) {
            let previous_state =
                slice::get(&reaction.previous.stories, story.id).map(|prior| prior.state);
            if previous_state.is_some_and(|prior| prior != story.state) {
                changed.push((story.id.value(), story.state.as_str().to_string()));
            }
        }
        let transitions = self.transitions.clone();
        Box::pin(async move {
            for (story_id, new_state) in changed {
                transitions.record(story_id, &new_state, observed_at);
            }
        })
    }
}
