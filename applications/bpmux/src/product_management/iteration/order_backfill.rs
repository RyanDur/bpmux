use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::domain::StoryType;
use crate::product_management::board::events::EventMeta;
use crate::product_management::board::{BoardEvent, PeerId, RefreshDriver};
use crate::product_management::iteration::label::{DEFAULT_FIRST_POSITION, DEFAULT_STEP};

pub fn backfill_iteration_order(orch: &dyn RefreshDriver) {
    let stories = orch.all_stories();
    let mut order_less: Vec<u64> = stories
        .iter()
        .filter(|s| {
            s.story_type == StoryType::Iteration && !s.is_closed && s.iteration_order.is_none()
        })
        .map(|s| s.number)
        .collect();
    if order_less.is_empty() {
        return;
    }
    order_less.sort_unstable();
    let mut next = stories
        .iter()
        .filter(|s| s.story_type == StoryType::Iteration && !s.is_closed)
        .filter_map(|s| s.iteration_order)
        .max()
        .map_or(DEFAULT_FIRST_POSITION, |m| m + DEFAULT_STEP);
    for number in order_less {
        let meta = EventMeta {
            actor: "system".to_string(),
            ts: chrono::Utc::now(),
            peer_id: PeerId::local(),
        };
        if let Err(e) = orch.mutate_sync(BoardEvent::TagAdded {
            meta,
            number,
            label: GhLabelCodec.encode_iteration_order(next),
        }) {
            crate::diag::log_diag(&format!(
                "iteration-order backfill: #{number} order-assign failed: {e}; idempotent retry next boot"
            ));
        }
        next += DEFAULT_STEP;
    }
}
