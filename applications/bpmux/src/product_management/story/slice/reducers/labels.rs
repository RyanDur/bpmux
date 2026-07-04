use super::Stories;
use crate::product_management::story::events::LabelDerivedFields;
use crate::product_management::story::{StoryId, StoryKey};

pub(super) fn apply_labels_refreshed(w: &mut Stories, id: StoryId, fields: &LabelDerivedFields) {
    if fields.raw_labels.is_empty() {
        return;
    }
    if let Some(existing) = w.get_mut(&StoryKey::Saved(id)) {
        existing.acceptor = fields.acceptor;
        existing.parent_epic = fields.parent_epic;
        existing.tags = fields.tags.clone();
        existing.blocked_by = fields.blocked_by;
        existing.ready_order = fields.ready_order;
        existing.iteration_slot = fields.iteration_slot.clone();
    }
}
