use super::status::{record_error, record_ingest_success};
use super::types::cache_fill_meta;
use super::{FreeComment, GateComment, SyncOrchestrator};
use crate::product_management::board::sync::{ConditionalBody, SourceAdapter, SyncError};
use crate::product_management::story::StoryId;
use crate::product_management::story::events::StoryEvent;

impl<A: SourceAdapter + 'static> SyncOrchestrator<A> {
    pub async fn fetch_body(&self, number: u64) -> Result<String, SyncError> {
        match self.adapter.fetch_story_body(number).await {
            Ok(body) => {
                let event = StoryEvent::BodyFetched {
                    meta: cache_fill_meta(),
                    id: StoryId::new(number),
                    body: body.clone(),
                    etag: None,
                };
                self.apply_story_event(&event);
                record_ingest_success(&self.status);
                Ok(body)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_body({number}): {e}"));
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
                record_ingest_success(&self.status);
                Ok(ConditionalBody::NotModified)
            }
            Ok(ConditionalBody::Modified { body, etag }) => {
                let event = StoryEvent::BodyFetched {
                    meta: cache_fill_meta(),
                    id: StoryId::new(number),
                    body: body.clone(),
                    etag: etag.clone(),
                };
                self.apply_story_event(&event);
                record_ingest_success(&self.status);
                Ok(ConditionalBody::Modified { body, etag })
            }
            Err(e) => {
                record_error(
                    &self.status,
                    &format!("fetch_body_conditional({number}): {e}"),
                );
                Err(e)
            }
        }
    }

    pub async fn fetch_free_comments(&self, number: u64) -> Result<Vec<FreeComment>, SyncError> {
        match self.adapter.fetch_free_comments(number).await {
            Ok(comments) => {
                let event = StoryEvent::FreeCommentsFetched {
                    meta: cache_fill_meta(),
                    id: StoryId::new(number),
                    comments: comments.clone(),
                };
                self.apply_story_event(&event);
                record_ingest_success(&self.status);
                Ok(comments)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_free_comments({number}): {e}"));
                Err(e)
            }
        }
    }

    pub async fn fetch_gate_comments(&self, number: u64) -> Result<Vec<GateComment>, SyncError> {
        match self.adapter.fetch_gate_comments(number).await {
            Ok(comments) => {
                let event = StoryEvent::GateCommentsFetched {
                    meta: cache_fill_meta(),
                    id: StoryId::new(number),
                    comments: comments.clone(),
                };
                self.apply_story_event(&event);
                record_ingest_success(&self.status);
                Ok(comments)
            }
            Err(e) => {
                record_error(&self.status, &format!("fetch_gate_comments({number}): {e}"));
                Err(e)
            }
        }
    }
}
