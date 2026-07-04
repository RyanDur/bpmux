use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Story, StoryId};
use crate::sync::{SourceAdapter, SyncError};

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

    pub(super) async fn update_body(&self, number: u64, body: String) -> Result<(), SyncError> {
        self.adapter.update_body(number, &body).await?;
        self.apply_story_event(&StoryEvent::BodyUpdated {
            meta: cache_fill_meta(),
            id: StoryId::new(number),
            body,
        });
        Ok(())
    }

    pub(super) async fn update_title(&self, number: u64, title: String) -> Result<(), SyncError> {
        self.adapter.update_title(number, &title).await?;
        self.apply_story_event(&StoryEvent::TitleUpdated {
            meta: cache_fill_meta(),
            id: StoryId::new(number),
            title,
        });
        Ok(())
    }
}
