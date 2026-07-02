use super::{AppAction, AppState};
use crate::product_management::story::slice::StoryLabelMiddleware;
use crate::store::kernel::Middleware;

pub(crate) struct StoryLabelAppMiddleware {
    inner: StoryLabelMiddleware,
}

impl StoryLabelAppMiddleware {
    pub(crate) fn new() -> Self {
        Self {
            inner: StoryLabelMiddleware,
        }
    }
}

impl Middleware<AppState, AppAction> for StoryLabelAppMiddleware {
    fn transform(&self, state: &AppState, action: AppAction) -> AppAction {
        match action {
            AppAction::Story(story_action) => {
                AppAction::Story(self.inner.transform(&state.stories, story_action))
            }
            other => other,
        }
    }
}
