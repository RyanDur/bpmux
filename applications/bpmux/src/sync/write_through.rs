use crate::sync::SyncError;
use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId};

use super::{SyncOrchestrator, cache_fill_meta};

impl SyncOrchestrator {
    pub(crate) fn remove_story(&self, id: StoryId) -> Option<Story> {
        let removed = self.read_story(id);
        self.dispatch_story(StoryAction::Removed(id));
        removed
    }

    pub(crate) fn clear_stories(&self) {
        self.dispatch_story(StoryAction::Cleared);
    }

    pub(crate) fn update_body(&self, number: u64, body: String) -> Result<(), SyncError> {
        self.mutate(StoryEvent::new(
            cache_fill_meta(),
            StoryId::new(number),
            StoryEventKind::BodyUpdated { body },
        ))
    }

    pub(crate) fn update_title(&self, number: u64, title: String) -> Result<(), SyncError> {
        self.mutate(StoryEvent::new(
            cache_fill_meta(),
            StoryId::new(number),
            StoryEventKind::TitleUpdated { title },
        ))
    }
}
