use super::ReactionsAction;
use crate::store::{Store, StoreReduceOutcome};

impl Store {
    pub(crate) fn middleware_reactions(&self, action: ReactionsAction) {
        let outcome = self.reactions_store.apply(&action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let channel = match &action {
            ReactionsAction::ReactionToggled { channel, .. } => channel.clone(),
        };
        let aggregated = self.reactions_store.aggregated_for_channel(&channel);
        let listeners = {
            let l = self
                .reactions_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&channel, &aggregated);
        }
    }
}
