use serde::Serialize;

use product_management::story::Stage;

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub async fn handle_dev_in_flight(select: &store::Select) -> Result<String, String> {
    let in_flight = select.select(|state| {
        product_management::story::slice::all(&state.stories)
            .iter()
            .any(|story| board::fine_stage(story, &state.board) == Some(Stage::Dev))
    });
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}
