use std::sync::Arc;

use kernel::{DeliveryMode, Listener, ListenerFuture, Reaction};
use store::{AppAction, AppState};

pub struct BoardBroadcast {
    hub: Arc<hub::Hub>,
}

impl BoardBroadcast {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

fn board_frames(previous: &AppState, current: &AppState) -> Vec<message::StateDeltaPayload> {
    use super::story_frames::{diff_story_frames, map_frames};
    let mut frames = diff_story_frames(
        &super::board::board_stories_json(previous),
        &super::board::board_stories_json(current),
    );
    frames.extend(map_frames(
        super::keyed_frames::diff_keyed(
            &super::board::board_epics_json(previous),
            &super::board::board_epics_json(current),
        ),
        |epic| message::StateDeltaPayload::EpicUpserted { epic },
        |id| message::StateDeltaPayload::EpicRemoved { id },
        |ids| message::StateDeltaPayload::EpicOrder { ids },
    ));
    frames.extend(map_frames(
        super::keyed_frames::diff_keyed(
            &super::board::board_outcomes_json(previous),
            &super::board::board_outcomes_json(current),
        ),
        |outcome| message::StateDeltaPayload::OutcomeUpserted { outcome },
        |id| message::StateDeltaPayload::OutcomeRemoved { id },
        |ids| message::StateDeltaPayload::OutcomeOrder { ids },
    ));
    frames.extend(map_frames(
        super::keyed_frames::diff_keyed(
            &super::board::board_iterations_json(previous),
            &super::board::board_iterations_json(current),
        ),
        |iteration| message::StateDeltaPayload::IterationUpserted { iteration },
        |id| message::StateDeltaPayload::IterationRemoved { id },
        |ids| message::StateDeltaPayload::IterationOrder { ids },
    ));
    frames.extend(map_frames(
        super::keyed_frames::diff_keyed(
            &super::board::board_roadmap_json(previous),
            &super::board::board_roadmap_json(current),
        ),
        |card| message::StateDeltaPayload::RoadmapCardUpserted { card },
        |id| message::StateDeltaPayload::RoadmapCardRemoved { id },
        |ids| message::StateDeltaPayload::RoadmapOrder { ids },
    ));
    frames
}

impl Listener<AppState, AppAction> for BoardBroadcast {
    fn on_action(&self, reaction: Reaction<AppState, AppAction>) -> ListenerFuture {
        let projection_input = matches!(
            reaction.action,
            AppAction::Story(_)
                | AppAction::Iteration(_)
                | AppAction::Board(_)
                | AppAction::Epic(_)
                | AppAction::Outcome(_)
        );
        if !projection_input {
            return Box::pin(async {});
        }
        let frames = board_frames(&reaction.previous, &reaction.current);
        let hub = Arc::clone(&self.hub);
        Box::pin(async move {
            let board_changed = !frames.is_empty();
            for frame in frames {
                hub.broadcast_ephemeral(&super::shared::state_delta_message(frame));
            }
            if board_changed {
                super::shared::broadcast_board_invalidate(&hub);
            }
        })
    }

    fn delivery_mode(&self) -> DeliveryMode {
        DeliveryMode::Ordered
    }
}
