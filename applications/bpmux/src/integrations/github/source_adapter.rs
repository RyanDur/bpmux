use super::{GitHubSourceAdapter, ingest, mutation};
use crate::bridge::tracker::{ConditionalIssue, StoryTracker};
use crate::integrations::github::mapper::{
    decode_free_comment, decode_gate_comments, decode_story_metadata,
};
use crate::product_management::board::sync::{ConditionalBody, Ingested, SourceAdapter, SyncError};
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{FreeComment, GateComment, Story};

impl SourceAdapter for GitHubSourceAdapter {
    async fn ingest_metadata(&self) -> Result<Ingested, SyncError> {
        self.ingest_metadata_since(None).await
    }

    async fn ingest_metadata_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Ingested, SyncError> {
        let values = self
            .inner
            .fetch_board_stories_since(since)
            .await
            .map_err(SyncError::Other)?;
        let received = values.len();
        let ingest::Classified {
            stories,
            outcomes,
            mut iterations,
            epics,
            dropped,
        } = ingest::classify_ingested(&values);
        let pre_filter = iterations.len();
        iterations.retain(|iteration| !(iteration.complete && iteration.stories.is_empty()));
        let filtered_out = pre_filter - iterations.len();
        crate::diag::log_diag(&format!(
            "github ingest_metadata_since(since={:?}): received {received} values, decoded {} stories + {} outcomes + {pre_filter} iterations + {} epics, dropped {dropped} (no number field), filtered {filtered_out} retired+0-member iterations",
            since.map(|s| s.to_rfc3339()),
            stories.len(),
            outcomes.len(),
            epics.len(),
        ));
        Ok(Ingested {
            items: stories,
            outcomes,
            iterations,
            epics,
        })
    }

    async fn fetch_story_metadata(&self, number: u64) -> Result<Option<Story>, SyncError> {
        match self.inner.get_story(number).await {
            Ok(Some(v)) => Ok(decode_story_metadata(&v)),
            Ok(None) => Ok(None),
            Err(e) => Err(SyncError::Other(e)),
        }
    }

    async fn fetch_story_body(&self, number: u64) -> Result<String, SyncError> {
        match self.inner.get_story(number).await {
            Ok(Some(v)) => Ok(v
                .get("body")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()),
            Ok(None) => Err(SyncError::Other(format!("story #{number} not found"))),
            Err(e) => Err(SyncError::Other(e)),
        }
    }

    async fn fetch_story_body_conditional(
        &self,
        number: u64,
        prior_etag: Option<String>,
    ) -> Result<ConditionalBody, SyncError> {
        match self
            .inner
            .get_story_conditional(number, prior_etag.as_deref())
            .await
        {
            Ok(ConditionalIssue::NotModified) => Ok(ConditionalBody::NotModified),
            Ok(ConditionalIssue::Modified { value, etag }) => {
                let body = value
                    .get("body")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(ConditionalBody::Modified { body, etag })
            }
            Ok(ConditionalIssue::NotFound) => {
                Err(SyncError::Other(format!("story #{number} not found")))
            }
            Err(e) => Err(SyncError::Other(e)),
        }
    }

    async fn fetch_story_labels(&self, number: u64) -> Result<Vec<String>, SyncError> {
        Ok(self.inner.get_story_labels(number).await)
    }

    async fn fetch_free_comments(&self, number: u64) -> Result<Vec<FreeComment>, SyncError> {
        let raw = self
            .inner
            .list_story_comments(number)
            .await
            .map_err(SyncError::Other)?;
        Ok(raw.iter().filter_map(decode_free_comment).collect())
    }

    async fn fetch_gate_comments(&self, number: u64) -> Result<Vec<GateComment>, SyncError> {
        let raw = self
            .inner
            .list_story_comments(number)
            .await
            .map_err(SyncError::Other)?;
        Ok(decode_gate_comments(&raw))
    }

    async fn push_event(&self, event: &StoryEvent) -> Result<Option<StoryEvent>, SyncError> {
        mutation::apply_push_event(&self.inner, event).await
    }

    async fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> Result<u64, SyncError> {
        let issue = self
            .inner
            .create_story(title, body, initial_labels)
            .await
            .map_err(SyncError::Other)?;
        issue.get("number").and_then(|v| v.as_u64()).ok_or_else(|| {
            SyncError::Other(format!(
                "create_story succeeded but issue number missing from response: {issue}"
            ))
        })
    }

    async fn update_body(&self, number: u64, body: &str) -> Result<Story, SyncError> {
        let value = self
            .inner
            .update_body(number, body)
            .await
            .map_err(SyncError::Other)?;
        decode_story_metadata(&value).ok_or_else(|| {
            SyncError::Other(format!(
                "update_body succeeded but response unparseable as Story: {value}"
            ))
        })
    }

    async fn update_title(&self, number: u64, title: &str) -> Result<Story, SyncError> {
        let value = self
            .inner
            .update_title(number, title)
            .await
            .map_err(SyncError::Other)?;
        decode_story_metadata(&value).ok_or_else(|| {
            SyncError::Other(format!(
                "update_title succeeded but response unparseable as Story: {value}"
            ))
        })
    }
}
