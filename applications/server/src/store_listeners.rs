use std::sync::{Arc, Mutex};

use store::{
    AppAction, AppState, CiBroadcast, CiListeners, ReactionsBroadcast, ReactionsListeners,
    RosterBroadcast, RosterListeners,
};
use transport::granular_broadcasts::{ReactionDeltaBroadcast, RosterDeltaBroadcast};
use transport::state_delta_broadcasters::CiHubBroadcaster;

pub(crate) struct BroadcastListeners {
    pub roster: RosterListeners,
    pub reactions: ReactionsListeners,
    pub ci: CiListeners,
}

pub(crate) fn wire_hub_broadcasters(
    app_store: &Arc<kernel::Store<AppState, AppAction>>,
    hub: &Arc<hub::Hub>,
    actions_url: &ci::ActionsUrl,
) -> BroadcastListeners {
    let roster: RosterListeners = Arc::new(Mutex::new(Vec::new()));
    let reactions: ReactionsListeners = Arc::new(Mutex::new(Vec::new()));
    app_store.add_listener(Arc::new(RosterDeltaBroadcast::new(hub.clone())));
    app_store.add_listener(Arc::new(ReactionDeltaBroadcast::new(hub.clone())));
    let ci: CiListeners = Arc::new(Mutex::new(vec![Arc::new(CiHubBroadcaster::new(
        hub.clone(),
        actions_url.clone(),
    ))]));
    app_store.add_listener(Arc::new(RosterBroadcast::new(roster.clone())));
    app_store.add_listener(Arc::new(ReactionsBroadcast::new(reactions.clone())));
    app_store.add_listener(Arc::new(CiBroadcast::new(ci.clone())));
    BroadcastListeners {
        roster,
        reactions,
        ci,
    }
}
