use crate::board::domain::{FreeComment, GateComment, Story};
use crate::board::events::BoardEvent;
use crate::board::sync::{ConditionalBody, SourceAdapter, SyncError};
use crate::bridge::tracker::{
    ConditionalIssue, ConditionalRunEntry, OctocrabStoryTracker, StoryTracker,
};
use crate::orchestrator::CiPollFetch;

mod codec;
pub use codec::*;
mod retired_filter;
pub use retired_filter::*;

pub struct GitHubSourceAdapter {
    inner: OctocrabStoryTracker,
}

impl CiPollFetch for GitHubSourceAdapter {
    fn list_latest_run_on_main_conditional(
        &self,
        prior_etag: Option<&str>,
    ) -> impl std::future::Future<Output = Result<ConditionalRunEntry, String>> + Send {
        GitHubSourceAdapter::list_latest_run_on_main_conditional(self, prior_etag)
    }

    fn rate_limit_observer(&self) -> crate::github::RateLimitObserver {
        GitHubSourceAdapter::rate_limit_observer(self)
    }
}

impl SourceAdapter for GitHubSourceAdapter {
    async fn ingest_metadata(&self) -> Result<Vec<Story>, SyncError> {
        self.ingest_metadata_since(None).await
    }

    async fn ingest_metadata_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<Story>, SyncError> {
        let values = self
            .inner
            .fetch_board_stories_since(since)
            .await
            .map_err(SyncError::Other)?;
        let received = values.len();
        let mut out = Vec::with_capacity(received);
        let mut dropped = 0usize;
        for v in values.iter() {
            if let Some(story) = decode_story_metadata(v) {
                out.push(story);
            } else {
                dropped += 1;
            }
        }
        let pre_filter = out.len();
        out = filter_retired_empty_iterations(out);
        let filtered_out = pre_filter - out.len();
        crate::session::diag::log_diag(&format!(
            "github ingest_metadata_since(since={:?}): received {received} values, decoded {pre_filter} stories, dropped {dropped} (no number field), filtered {filtered_out} retired+0-member iterations",
            since.map(|s| s.to_rfc3339()),
        ));
        Ok(out)
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

    async fn push_event(&self, event: &BoardEvent) -> Result<Option<BoardEvent>, SyncError> {
        match event {
            BoardEvent::StoryCreated { .. } => Ok(None),
            BoardEvent::StateTransitioned {
                number, new_state, ..
            } => {
                let label = state_to_label(*new_state);
                let labels = self
                    .inner
                    .add_label(*number, label)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(Some(BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::StoryClosed { number, reason, .. } => {
                if let Some(reason_text) = reason.as_deref() {
                    self.inner
                        .comment_on_story(*number, reason_text)
                        .await
                        .map_err(SyncError::Other)?;
                }
                match self.inner.close_story(*number).await {
                    Ok(value) => {
                        Ok(
                            decode_story_metadata(&value).map(|story| BoardEvent::StoryRefreshed {
                                meta: crate::board::orchestrator::cache_fill_meta(),
                                number: *number,
                                story: Box::new(story),
                            }),
                        )
                    }
                    Err(e) => match crate::board::sync::classify_close_error(e) {
                        SyncError::AlreadyClosed => Ok(None),
                        other => Err(other),
                    },
                }
            }
            BoardEvent::StoryReopened { number, .. } => {
                let value = self
                    .inner
                    .reopen_story(*number)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(
                    decode_story_metadata(&value).map(|story| BoardEvent::StoryRefreshed {
                        meta: crate::board::orchestrator::cache_fill_meta(),
                        number: *number,
                        story: Box::new(story),
                    }),
                )
            }
            BoardEvent::GateCommentAppended {
                meta,
                number,
                gate,
                payload,
            } => {
                let body = encode_gate_comment(*gate, meta, payload.as_deref());
                self.inner
                    .comment_on_story(*number, &body)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(None)
            }
            BoardEvent::FreeCommentAppended { number, body, .. } => {
                self.inner
                    .comment_on_story(*number, body)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(None)
            }
            BoardEvent::TagAdded { number, label, .. } => {
                let labels = self
                    .inner
                    .add_label(*number, label)
                    .await
                    .map_err(SyncError::Other)?;
                if is_position_label(label) {
                    Ok(None)
                } else {
                    Ok(Some(BoardEvent::LabelsRefreshed {
                        meta: crate::board::orchestrator::cache_fill_meta(),
                        number: *number,
                        labels,
                    }))
                }
            }
            BoardEvent::TagRemoved { number, label, .. } => {
                let labels = self
                    .inner
                    .remove_label(*number, label)
                    .await
                    .map_err(SyncError::Other)?;
                if is_position_label(label) {
                    Ok(None)
                } else {
                    Ok(Some(BoardEvent::LabelsRefreshed {
                        meta: crate::board::orchestrator::cache_fill_meta(),
                        number: *number,
                        labels,
                    }))
                }
            }
            BoardEvent::ParentEpicSet {
                number,
                epic_number,
                ..
            } => {
                let label = format!("parent-epic-#{epic_number}");
                let labels = self
                    .inner
                    .add_label(*number, &label)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(Some(BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::ParentEpicUnset { number, .. } => {
                let current = self.inner.get_story_labels(*number).await;
                let mut last_response: Option<Vec<String>> = None;
                for l in current.iter().filter(|l| l.starts_with("parent-epic-#")) {
                    let labels = self
                        .inner
                        .remove_label(*number, l)
                        .await
                        .map_err(SyncError::Other)?;
                    last_response = Some(labels);
                }
                Ok(last_response.map(|labels| BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::IterationMembershipSet {
                number,
                iteration_number,
                position,
                ..
            } => {
                let current = self.inner.get_story_labels(*number).await;
                for l in current
                    .iter()
                    .filter(|l| l.starts_with("parent-iteration-#"))
                {
                    self.inner
                        .remove_label(*number, l)
                        .await
                        .map_err(SyncError::Other)?;
                }
                let label = format!("parent-iteration-#{iteration_number}.{position}");
                let labels = self
                    .inner
                    .add_label(*number, &label)
                    .await
                    .map_err(SyncError::Other)?;
                Ok(Some(BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::CurrentIterationDesignated { number, .. } => {
                let labels = self
                    .inner
                    .add_label(
                        *number,
                        crate::board::adapters::gh_label_codec::CURRENT_ITERATION_LABEL,
                    )
                    .await
                    .map_err(SyncError::Other)?;
                Ok(Some(BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::CurrentIterationCleared { number, .. } => {
                let labels = self
                    .inner
                    .remove_label(
                        *number,
                        crate::board::adapters::gh_label_codec::CURRENT_ITERATION_LABEL,
                    )
                    .await
                    .map_err(SyncError::Other)?;
                Ok(Some(BoardEvent::LabelsRefreshed {
                    meta: crate::board::orchestrator::cache_fill_meta(),
                    number: *number,
                    labels,
                }))
            }
            BoardEvent::BodyFetched { .. }
            | BoardEvent::FreeCommentsFetched { .. }
            | BoardEvent::GateCommentsFetched { .. }
            | BoardEvent::StoryRefreshed { .. }
            | BoardEvent::LabelsRefreshed { .. }
            | BoardEvent::ConflictDetected { .. }
            | BoardEvent::ConflictResolved { .. } => Ok(None),
        }
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

mod delegates;
