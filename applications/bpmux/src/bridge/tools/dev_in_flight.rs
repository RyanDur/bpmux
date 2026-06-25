use std::sync::Arc;

use serde::Serialize;

use crate::product_management::board::BoardStore;
use crate::product_management::story::StoryState;

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub(in crate::bridge) async fn handle_dev_in_flight(
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let in_flight = !store.stories_in_state(StoryState::InProgress).is_empty();
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}
