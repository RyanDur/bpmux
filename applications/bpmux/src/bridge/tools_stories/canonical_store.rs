use crate::product_management::story::StoryId;

pub(crate) async fn ensure_story_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
) -> Result<crate::product_management::story::Story, String> {
    if let Some(story) = refresh.story(number) {
        return Ok(story);
    }
    refresh.refresh_story(number).await.map_err(|e| {
        format!(
            "story #{number} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown story state"
        )
    })?;
    refresh.story(number).ok_or_else(|| {
        format!(
            "story #{number} not found in canonical store after refresh_story \
             round-trip; story may not exist at source-of-truth"
        )
    })
}

pub(crate) async fn ensure_outcome_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
) -> Result<crate::product_management::outcome::Outcome, String> {
    if let Some(outcome) = find_outcome_in_store(refresh, number) {
        return Ok(outcome);
    }
    if refresh.story(number).is_some() {
        return Err(not_an_outcome_message(number));
    }
    refresh.refresh_story(number).await.map_err(|e| {
        format!(
            "outcome #{number} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown outcome state"
        )
    })?;
    if let Some(outcome) = find_outcome_in_store(refresh, number) {
        return Ok(outcome);
    }
    if refresh.story(number).is_some() {
        return Err(not_an_outcome_message(number));
    }
    Err(format!(
        "outcome #{number} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn find_outcome_in_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
) -> Option<crate::product_management::outcome::Outcome> {
    refresh
        .outcomes()
        .into_iter()
        .find(|outcome| outcome.id().saved_id() == Some(StoryId::new(number)))
}

fn not_an_outcome_message(number: u64) -> String {
    format!(
        "#{number} is not an Outcome; it may be a story, not an outcome — outcomes: bpmux roadmap"
    )
}

pub(crate) async fn ensure_iteration_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    number: u64,
) -> Result<crate::product_management::iteration::Iteration, String> {
    if let Some(iteration) = refresh.iteration(
        crate::product_management::iteration::IterationId::new(number),
    ) {
        return Ok(iteration);
    }
    if refresh.story(number).is_some() {
        return Err(not_an_iteration_message(number));
    }
    refresh.refresh_story(number).await.map_err(|e| {
        format!(
            "iteration #{number} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with an iteration operation against unknown state"
        )
    })?;
    if let Some(iteration) = refresh.iteration(
        crate::product_management::iteration::IterationId::new(number),
    ) {
        return Ok(iteration);
    }
    if refresh.story(number).is_some() {
        return Err(not_an_iteration_message(number));
    }
    Err(format!(
        "iteration #{number} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn not_an_iteration_message(number: u64) -> String {
    format!(
        "#{number} is not an Iteration; only iterations carry this operation — iterations: bpmux board"
    )
}
