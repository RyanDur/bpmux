use super::*;
use crate::board::events::BoardEvent;
use crate::board::sync::SyncError;
use crate::bridge::tracker::{OctocrabStoryTracker, StoryTracker};

pub(super) async fn apply_push_event(
    inner: &OctocrabStoryTracker,
    event: &BoardEvent,
) -> Result<Option<BoardEvent>, SyncError> {
    match event {
        BoardEvent::StoryCreated { .. } => Ok(None),
        BoardEvent::StateTransitioned {
            number, new_state, ..
        } => {
            let label = state_to_label(*new_state);
            let labels = inner
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
                inner
                    .comment_on_story(*number, reason_text)
                    .await
                    .map_err(SyncError::Other)?;
            }
            match inner.close_story(*number).await {
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
            let value = inner
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
            inner
                .comment_on_story(*number, &body)
                .await
                .map_err(SyncError::Other)?;
            Ok(None)
        }
        BoardEvent::FreeCommentAppended { number, body, .. } => {
            inner
                .comment_on_story(*number, body)
                .await
                .map_err(SyncError::Other)?;
            Ok(None)
        }
        BoardEvent::TagAdded { number, label, .. } => {
            let labels = inner
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
            let labels = inner
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
            let labels = inner
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
            let current = inner.get_story_labels(*number).await;
            let mut last_response: Option<Vec<String>> = None;
            for l in current.iter().filter(|l| l.starts_with("parent-epic-#")) {
                let labels = inner
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
            let current = inner.get_story_labels(*number).await;
            for l in current
                .iter()
                .filter(|l| l.starts_with("parent-iteration-#"))
            {
                inner
                    .remove_label(*number, l)
                    .await
                    .map_err(SyncError::Other)?;
            }
            let label = format!("parent-iteration-#{iteration_number}.{position}");
            let labels = inner
                .add_label(*number, &label)
                .await
                .map_err(SyncError::Other)?;
            Ok(Some(BoardEvent::LabelsRefreshed {
                meta: crate::board::orchestrator::cache_fill_meta(),
                number: *number,
                labels,
            }))
        }
        BoardEvent::OutcomeMembershipSet {
            number,
            outcome_number,
            position,
            ..
        } => {
            let current = inner.get_story_labels(*number).await;
            for l in current.iter().filter(|l| l.starts_with("parent-outcome-#")) {
                inner
                    .remove_label(*number, l)
                    .await
                    .map_err(SyncError::Other)?;
            }
            let label = format!("parent-outcome-#{outcome_number}.{position}");
            let labels = inner
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
            let labels = inner
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
            let labels = inner
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
