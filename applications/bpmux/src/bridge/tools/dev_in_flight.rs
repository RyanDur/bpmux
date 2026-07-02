use std::sync::Arc;

use serde::Serialize;

use crate::product_management::board::RefreshDriver;
use crate::product_management::story::{BoardState, Stage};

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub(in crate::bridge) async fn handle_dev_in_flight(
    refresh: &Arc<dyn RefreshDriver>,
) -> Result<String, String> {
    let in_flight = refresh
        .all_stories()
        .iter()
        .any(|s| s.state() == BoardState::InProgress(Stage::Dev));
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}
