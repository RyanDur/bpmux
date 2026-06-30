use crate::product_management::story::StoryId;

pub(crate) async fn ensure_story_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
) -> Result<crate::product_management::story::Story, String> {
    if let Some(story) = refresh.story(id) {
        return Ok(story);
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "story #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown story state"
        )
    })?;
    refresh.story(id).ok_or_else(|| {
        format!(
            "story #{id} not found in canonical store after refresh_story \
             round-trip; story may not exist at source-of-truth"
        )
    })
}

pub(crate) async fn ensure_outcome_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
) -> Result<crate::product_management::outcome::Outcome, String> {
    if let Some(outcome) = find_outcome_in_store(refresh, id) {
        return Ok(outcome);
    }
    if refresh.story(id).is_some() {
        return Err(not_an_outcome_message(id));
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "outcome #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown outcome state"
        )
    })?;
    if let Some(outcome) = find_outcome_in_store(refresh, id) {
        return Ok(outcome);
    }
    if refresh.story(id).is_some() {
        return Err(not_an_outcome_message(id));
    }
    Err(format!(
        "outcome #{id} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn find_outcome_in_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
) -> Option<crate::product_management::outcome::Outcome> {
    refresh
        .outcomes()
        .into_iter()
        .find(|outcome| outcome.id().saved_id() == Some(StoryId::new(id)))
}

fn not_an_outcome_message(id: u64) -> String {
    format!("#{id} is not an Outcome; it may be a story, not an outcome — outcomes: bpmux roadmap")
}

pub(crate) async fn ensure_iteration_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    id: u64,
) -> Result<crate::product_management::iteration::Iteration, String> {
    if let Some(iteration) =
        refresh.iteration(crate::product_management::iteration::IterationId::new(id))
    {
        return Ok(iteration);
    }
    if refresh.story(id).is_some() {
        return Err(not_an_iteration_message(id));
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "iteration #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with an iteration operation against unknown state"
        )
    })?;
    if let Some(iteration) =
        refresh.iteration(crate::product_management::iteration::IterationId::new(id))
    {
        return Ok(iteration);
    }
    if refresh.story(id).is_some() {
        return Err(not_an_iteration_message(id));
    }
    Err(format!(
        "iteration #{id} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn not_an_iteration_message(id: u64) -> String {
    format!(
        "#{id} is not an Iteration; only iterations carry this operation — iterations: bpmux board"
    )
}
