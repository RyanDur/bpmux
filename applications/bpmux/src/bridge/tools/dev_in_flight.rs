use serde::Serialize;

use product_management::story::Stage;

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub(in crate::bridge) async fn handle_dev_in_flight(
    select: &crate::store::Select,
) -> Result<String, String> {
    let in_flight = select.select(|s| {
        product_management::story::slice::all(&s.stories)
            .iter()
            .any(|story| board::fine_stage(story, &s.board) == Some(Stage::Dev))
    });
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}
