use chrono::{DateTime, Utc};

use crate::board::domain::{FreeComment, Gate, GateComment, Story, StoryState, Tag};
use crate::board::events::{BoardEvent, EventMeta};
use crate::board::sync::{ConditionalBody, SourceAdapter, SyncError};
use crate::bridge::tracker::{
    ConditionalIssue, ConditionalRunEntry, OctocrabStoryTracker, StoryTracker,
};
use crate::orchestrator::CiPollFetch;

pub struct GitHubSourceAdapter {
    inner: OctocrabStoryTracker,
}

impl GitHubSourceAdapter {
    pub fn new(inner: OctocrabStoryTracker) -> Self {
        Self { inner }
    }

    pub fn from_env() -> Option<Self> {
        OctocrabStoryTracker::from_env().map(Self::new)
    }

    pub async fn accept_story(&self, number: u64) -> Result<(), String> {
        self.inner.accept_story(number).await
    }

    pub async fn reject_story(&self, number: u64, note: &str) -> Result<(), String> {
        self.inner.reject_story(number, note).await
    }

    pub async fn post_story_comment(&self, number: u64, body: &str) -> Result<(), String> {
        self.inner.post_story_comment(number, body).await
    }

    pub async fn list_story_timeline(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        self.inner.list_story_timeline(number).await
    }

    pub async fn list_latest_run_on_main(
        &self,
    ) -> Result<Option<crate::bridge::tracker::WorkflowRunEntry>, String> {
        self.inner.list_latest_run_on_main().await
    }

    pub async fn list_latest_run_on_main_conditional(
        &self,
        prior_etag: Option<&str>,
    ) -> Result<crate::bridge::tracker::ConditionalRunEntry, String> {
        self.inner
            .list_latest_run_on_main_conditional(prior_etag)
            .await
    }

    pub async fn fetch_failed_tests_for_run(
        &self,
        run_id: u64,
    ) -> Result<crate::bridge::tools::CiRunFailures, crate::bridge::tools::CiLogError> {
        self.inner.fetch_failed_tests_for_run(run_id).await
    }

    pub async fn rerun_failed_jobs(&self, run_id: u64) -> Result<(), String> {
        self.inner.rerun_failed_jobs(run_id).await
    }

    pub fn rate_limit_observer(&self) -> crate::github::RateLimitObserver {
        self.inner.rate_limit_observer()
    }

    pub async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
        self.inner.get_story(number).await
    }

    pub async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        self.inner.fetch_board_stories().await
    }

    pub async fn list_story_comments(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        self.inner.list_story_comments(number).await
    }
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
            BoardEvent::BodyFetched { .. }
            | BoardEvent::FreeCommentsFetched { .. }
            | BoardEvent::GateCommentsFetched { .. }
            | BoardEvent::StoryRefreshed { .. }
            | BoardEvent::LabelsRefreshed { .. }
            | BoardEvent::ConflictDetected { .. }
            | BoardEvent::ConflictResolved { .. } => Ok(None),
        }
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<u64, SyncError> {
        let issue = self
            .inner
            .create_story(title, body)
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

pub fn warn_on_inconsistent_state_labels(labels: &[String]) {
    const STATE_LABELS: &[&str] = &[
        "ready",
        "in progress",
        "desk-check",
        "delivered",
        "ready for accept",
        "needs fixes",
        "rejected",
        "accepted",
    ];
    let present: Vec<&str> = STATE_LABELS
        .iter()
        .copied()
        .filter(|needle| labels.iter().any(|l| l.eq_ignore_ascii_case(needle)))
        .collect();
    if present.len() >= 2 {
        crate::session::diag::log_diag(&format!(
            "decoder: inconsistent state-labels present {present:?} — priority decoder picks first per spec; expected: only one state-label per story"
        ));
    }
}

pub fn filter_retired_empty_iterations(stories: Vec<Story>) -> Vec<Story> {
    let purge: Vec<u64> = iteration_numbers_to_purge_as_retired_empty(&stories);
    if purge.is_empty() {
        return stories;
    }
    stories
        .into_iter()
        .filter(|s| !purge.contains(&s.number))
        .collect()
}

pub fn iteration_numbers_to_purge_as_retired_empty(stories: &[Story]) -> Vec<u64> {
    use crate::board::domain::StoryType;
    let retired_iterations: Vec<(&Story, String)> = stories
        .iter()
        .filter(|s| s.story_type == StoryType::Iteration && s.state == StoryState::Retired)
        .filter_map(|s| {
            s.title
                .strip_prefix("Iteration ")
                .map(|theme| (s, theme.to_string()))
        })
        .collect();
    if retired_iterations.is_empty() {
        return Vec::new();
    }
    retired_iterations
        .into_iter()
        .filter(|(_, theme)| {
            let prefix = format!("iteration-{theme}.");
            !stories
                .iter()
                .any(|s| s.raw_labels.iter().any(|l| l.starts_with(&prefix)))
        })
        .map(|(s, _)| s.number)
        .collect()
}

pub fn decode_story_metadata(v: &serde_json::Value) -> Option<Story> {
    let number = v.get("number").and_then(|x| x.as_u64())?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let labels = extract_label_names(v);
    let is_closed = match v.get("gh_state").and_then(|x| x.as_str()) {
        Some(s) => matches!(s, "closed" | "CLOSED"),
        None => matches!(
            v.get("state").and_then(|x| x.as_str()),
            Some("closed" | "CLOSED")
        ),
    };
    let state = state_from_labels(&labels, is_closed);
    let filed_at = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    let filed_by = v
        .get("user")
        .and_then(|u| u.get("login"))
        .and_then(|x| x.as_str())
        .map(ToString::to_string)
        .unwrap_or_else(|| "unknown".to_string());
    let mut story = Story::new_minimal(number, title.clone(), state, filed_at, filed_by);
    story.story_type = crate::board::domain::story_type_from_labels_or_title(&labels, &title);
    story.tags = tags_from_labels(&labels);
    story.is_closed = is_closed;
    story.parent_epic = parent_epic_from_labels(&labels);
    story.raw_labels = labels;
    if let Some(body) = v.get("body").and_then(|x| x.as_str()) {
        story.body = Some(body.to_string());
        story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
    }
    Some(story)
}

pub fn parent_epic_from_labels(labels: &[String]) -> Option<u64> {
    labels
        .iter()
        .find_map(|l| l.strip_prefix("parent-epic-#").and_then(|n| n.parse().ok()))
}

fn is_iteration_position_label(label: &str) -> bool {
    let Some(rest) = label.strip_prefix("iteration-") else {
        return false;
    };
    if rest.starts_with("order.") {
        return false;
    }
    let Some(dot_idx) = rest.rfind('.') else {
        return false;
    };
    rest.get(dot_idx + 1..)
        .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()))
}

fn is_position_label(label: &str) -> bool {
    use crate::board::label_codec::LabelCodec;
    is_iteration_position_label(label)
        || crate::board::adapters::gh_label_codec::GhLabelCodec.is_ready_order_label(label)
}

#[cfg(test)]
pub(crate) fn is_iteration_position_label_for_test(label: &str) -> bool {
    is_iteration_position_label(label)
}

#[cfg(test)]
pub(crate) fn is_position_label_for_test(label: &str) -> bool {
    is_position_label(label)
}

fn extract_label_names(v: &serde_json::Value) -> Vec<String> {
    let Some(labels) = v.get("labels").and_then(|x| x.as_array()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::with_capacity(labels.len());
    for label in labels {
        if let Some(s) = label.as_str() {
            names.push(s.to_string());
        } else if let Some(n) = label.get("name").and_then(|x| x.as_str()) {
            names.push(n.to_string());
        }
    }
    names
}

pub fn tags_from_labels(labels: &[String]) -> Vec<Tag> {
    let mut out = Vec::new();
    for l in labels {
        if l.eq_ignore_ascii_case("blocked") && !out.contains(&Tag::Blocked) {
            out.push(Tag::Blocked);
        }
    }
    out
}

pub fn state_from_labels(labels: &[String], is_closed: bool) -> StoryState {
    let has = |needle: &str| labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
    warn_on_inconsistent_state_labels(labels);
    if has("accepted") {
        return StoryState::Accepted;
    }
    if has("retired") {
        return StoryState::Retired;
    }
    if is_closed {
        return StoryState::Accepted;
    }
    if has("rejected") {
        return StoryState::Ready;
    }
    if has("needs fixes") {
        return StoryState::InProgress;
    }
    if has("ready for accept") {
        return StoryState::Delivered;
    }
    if has("delivered") {
        return StoryState::Delivered;
    }
    if has("desk-check") {
        return StoryState::DeskCheck;
    }
    if has("in progress") {
        return StoryState::InProgress;
    }
    if has("ready") {
        return StoryState::Ready;
    }
    StoryState::Icebox
}

pub fn state_to_label(state: StoryState) -> &'static str {
    match state {
        StoryState::Icebox => "icebox",
        StoryState::Ready => "ready",
        StoryState::InProgress => "in progress",
        StoryState::DeskCheck => "desk-check",
        StoryState::Delivered => "delivered",
        StoryState::Accepted => "accepted",
        StoryState::Retired => "retired",
    }
}

fn decode_free_comment(v: &serde_json::Value) -> Option<FreeComment> {
    let body = v.get("body").and_then(|x| x.as_str())?.to_string();
    let author = v
        .get("author")
        .and_then(|x| x.as_str())
        .unwrap_or("unknown")
        .to_string();
    let ts = v
        .get("created_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);
    Some(FreeComment { body, author, ts })
}

fn encode_gate_comment(gate: Gate, meta: &EventMeta, payload: Option<&str>) -> String {
    let (marker, tag) = match gate {
        Gate::Pickup => ("pickup_paraphrase", "dev pickup-gate"),
        Gate::Delivered => ("verified_observation", "dev deliver-gate"),
        Gate::PassDeskCheck => ("pass_desk_check", "desk-check-pass"),
        Gate::FailDeskCheck => ("rejection_reason", "desk-check-fail"),
        Gate::Rejected => ("rejection_reason", "reject-gate"),
        Gate::Accepted => ("accepted", "accept-gate"),
        Gate::Filed => ("filed", "file-gate"),
    };
    match payload {
        Some(body) => format!("**{marker}** ({tag} by {}):\n\n{body}", meta.actor),
        None => format!("**{marker}** ({tag} by {})", meta.actor),
    }
}

pub fn decode_gate_comments(comments: &[serde_json::Value]) -> Vec<GateComment> {
    comments
        .iter()
        .filter_map(|c| {
            let body = c.get("body").and_then(|x| x.as_str())?;
            let author = c
                .get("author")
                .and_then(|x| x.as_str())
                .unwrap_or("unknown")
                .to_string();
            let ts = c
                .get("created_at")
                .and_then(|x| x.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))?;
            classify_gate(body).map(|(gate, payload)| GateComment {
                gate,
                ts,
                actor: author,
                payload,
            })
        })
        .collect()
}

pub fn classify_gate(body: &str) -> Option<(Gate, Option<String>)> {
    if body.contains("**pickup_paraphrase**") {
        return Some((Gate::Pickup, extract_marker_payload(body)));
    }
    if body.contains("**verified_observation**") {
        return Some((Gate::Delivered, extract_marker_payload(body)));
    }
    if body.contains("**pass_desk_check**") {
        return Some((Gate::PassDeskCheck, None));
    }
    if body.contains("**accepted**") {
        return Some((Gate::Accepted, None));
    }
    if body.contains("**rejection_reason**") {
        let gate = if body.contains("desk-check-fail") {
            Gate::FailDeskCheck
        } else {
            Gate::Rejected
        };
        return Some((gate, extract_marker_payload(body)));
    }
    None
}

fn extract_marker_payload(body: &str) -> Option<String> {
    body.split_once(":\n\n").map(|(_, rest)| rest.to_string())
}

#[cfg(test)]
mod filter_retired_empty_iterations_tests {
    use super::*;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::Utc;

    fn iter_story(number: u64, theme: &str, state: StoryState) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("Iteration {theme}"),
            state,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Iteration;
        s
    }

    fn member_story(number: u64, theme: &str, position: u64) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            StoryState::Icebox,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Feature;
        s.raw_labels.push(format!("iteration-{theme}.{position}"));
        s
    }

    #[test]
    fn retired_zero_member_iteration_filtered_out() {
        let stories = vec![iter_story(1, "Aquila", StoryState::Retired)];
        let filtered = filter_retired_empty_iterations(stories);
        assert!(
            filtered.is_empty(),
            "retired+0-member iteration must be filtered; got {filtered:?}"
        );
    }

    #[test]
    fn retired_with_members_preserved_history_value() {
        let stories = vec![
            iter_story(1, "Andromeda", StoryState::Retired),
            member_story(2, "Andromeda", 100),
            member_story(3, "Andromeda", 200),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 3, "closed+N kept; got {filtered:?}");
        assert!(filtered.iter().any(|s| s.number == 1));
    }

    #[test]
    fn open_zero_member_iteration_preserved_per_1413_fix() {
        let stories = vec![iter_story(1, "Lyra", StoryState::Icebox)];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 1, "open+0 stays per #1413 fix");
    }

    #[test]
    fn open_with_members_preserved() {
        let stories = vec![
            iter_story(1, "Carina", StoryState::Icebox),
            member_story(2, "Carina", 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn closed_accepted_with_members_preserved() {
        let stories = vec![
            iter_story(1, "Pegasus", StoryState::Accepted),
            member_story(2, "Pegasus", 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(
            filtered.len(),
            2,
            "Accepted (not Retired) iteration not subject to filter"
        );
    }

    #[test]
    fn mix_of_retired_empty_and_retired_with_members_filters_only_empty() {
        let stories = vec![
            iter_story(1, "Empty", StoryState::Retired),
            iter_story(2, "WithKids", StoryState::Retired),
            member_story(3, "WithKids", 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|s| s.number == 2));
        assert!(filtered.iter().any(|s| s.number == 3));
        assert!(!filtered.iter().any(|s| s.number == 1));
    }

    #[test]
    fn non_iteration_retired_story_not_subject_to_filter() {
        let mut s = Story::new_minimal(
            1,
            "[Chore] retired-chore".to_string(),
            StoryState::Retired,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Chore;
        let stories = vec![s];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 1, "only iteration-type-class is filtered");
    }
}
