use super::cache_fill_meta;
use super::{CastVerdict, FreeComment, SyncOrchestrator};
use crate::sync::{ConditionalBody, SourceAdapter, SyncError};
use product_management::story::StoryId;
use product_management::story::events::{StoryEvent, StoryEventKind};

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn fetch_body(&self, number: u64) -> Result<String, SyncError> {
        match self.adapter.fetch_story_body(number).await {
            Ok(body) => {
                let event = StoryEvent::new(
                    cache_fill_meta(),
                    StoryId::new(number),
                    StoryEventKind::BodyFetched {
                        body: body.clone(),
                        etag: None,
                    },
                );
                self.apply_story_event(&event);
                self.record_ingest_success();
                Ok(body)
            }
            Err(e) => {
                self.record_error(&format!("fetch_body({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> Result<ConditionalBody, SyncError> {
        match self
            .adapter
            .fetch_story_body_conditional(number, prior_etag)
            .await
        {
            Ok(ConditionalBody::NotModified) => {
                self.record_ingest_success();
                Ok(ConditionalBody::NotModified)
            }
            Ok(ConditionalBody::Modified { body, etag }) => {
                let event = StoryEvent::new(
                    cache_fill_meta(),
                    StoryId::new(number),
                    StoryEventKind::BodyFetched {
                        body: body.clone(),
                        etag: etag.clone(),
                    },
                );
                self.apply_story_event(&event);
                self.record_ingest_success();
                Ok(ConditionalBody::Modified { body, etag })
            }
            Err(e) => {
                self.record_error(&format!("fetch_body_conditional({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_free_comments(&self, number: u64) -> Result<Vec<FreeComment>, SyncError> {
        match self.adapter.fetch_free_comments(number).await {
            Ok(comments) => {
                let event = StoryEvent::new(
                    cache_fill_meta(),
                    StoryId::new(number),
                    StoryEventKind::FreeCommentsFetched {
                        comments: comments.clone(),
                    },
                );
                self.apply_story_event(&event);
                self.record_ingest_success();
                Ok(comments)
            }
            Err(e) => {
                self.record_error(&format!("fetch_free_comments({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_verdicts(&self, number: u64) -> Result<Vec<CastVerdict>, SyncError> {
        match self.adapter.fetch_verdicts(number).await {
            Ok(verdicts) => {
                let event = StoryEvent::new(
                    cache_fill_meta(),
                    StoryId::new(number),
                    StoryEventKind::VerdictsFetched {
                        verdicts: verdicts.clone(),
                    },
                );
                self.apply_story_event(&event);
                self.record_ingest_success();
                Ok(verdicts)
            }
            Err(e) => {
                self.record_error(&format!("fetch_verdicts({number}): {e}"));
                Err(e)
            }
        }
    }
}
