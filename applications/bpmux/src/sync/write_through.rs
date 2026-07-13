use crate::sync::{SourceAdapter, SyncError};
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId};

use super::{SyncOrchestrator, cache_fill_meta};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub(super) fn remove_story(&self, id: StoryId) -> Option<Story> {
        let removed = self.read_story(id);
        self.dispatch_story(StoryAction::Removed(id));
        removed
    }

    pub(super) fn clear_stories(&self) {
        self.dispatch_story(StoryAction::Cleared);
    }

    pub(super) fn update_body(&self, number: u64, body: String) -> Result<(), SyncError> {
        self.mutate(StoryEvent::new(
            cache_fill_meta(),
            StoryId::new(number),
            StoryEventKind::BodyUpdated { body },
        ))
    }

    pub(super) fn update_title(&self, number: u64, title: String) -> Result<(), SyncError> {
        self.mutate(StoryEvent::new(
            cache_fill_meta(),
            StoryId::new(number),
            StoryEventKind::TitleUpdated { title },
        ))
    }
}
