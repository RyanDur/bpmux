pub(crate) async fn ensure_story_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    id: u64,
) -> Result<product_management::story::Story, String> {
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
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    select: &crate::store::Select,
    id: u64,
) -> Result<product_management::outcome::Outcome, String> {
    if let Some(outcome) = find_outcome_in_store(select, id) {
        return Ok(outcome);
    }
    if backing_story_via_select(select, id).is_some() {
        return Err(not_an_outcome_message(id));
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "outcome #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with mutation against unknown outcome state"
        )
    })?;
    if let Some(outcome) = find_outcome_in_store(select, id) {
        return Ok(outcome);
    }
    if backing_story_via_select(select, id).is_some() {
        return Err(not_an_outcome_message(id));
    }
    Err(format!(
        "outcome #{id} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn find_outcome_in_store(
    select: &crate::store::Select,
    id: u64,
) -> Option<product_management::outcome::Outcome> {
    select
        .select(|s| product_management::outcome::all(&s.outcomes))
        .into_iter()
        .find(|outcome| outcome.id == product_management::outcome::OutcomeId::new(id))
}

fn backing_story_via_select(
    select: &crate::store::Select,
    id: u64,
) -> Option<product_management::story::Story> {
    select.select(|s| {
        product_management::story::slice::get(
            &s.stories,
            product_management::story::StoryId::new(id),
        )
    })
}

fn not_an_outcome_message(id: u64) -> String {
    format!("#{id} is not an Outcome; it may be a story, not an outcome — outcomes: bpmux roadmap")
}

pub(crate) async fn ensure_iteration_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    id: u64,
) -> Result<product_management::iteration::Iteration, String> {
    if let Some(iteration) = refresh.iteration(product_management::iteration::IterationId::new(id))
    {
        return Ok(iteration);
    }
    if refresh.backing_story(id).is_some() {
        return Err(not_an_iteration_message(id));
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "iteration #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with an iteration operation against unknown state"
        )
    })?;
    if let Some(iteration) = refresh.iteration(product_management::iteration::IterationId::new(id))
    {
        return Ok(iteration);
    }
    if refresh.backing_story(id).is_some() {
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

pub(crate) async fn ensure_epic_in_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    select: &crate::store::Select,
    id: u64,
) -> Result<product_management::epic::Epic, String> {
    let epic_id = product_management::epic::EpicId::new(id);
    if let Some(epic) = select.select(|s| product_management::epic::get(&s.epics, epic_id)) {
        return Ok(epic);
    }
    if backing_story_via_select(select, id).is_some() {
        return Err(not_an_epic_message(id));
    }
    refresh.refresh_story(id).await.map_err(|e| {
        format!(
            "epic #{id} not in canonical store + refresh_story failed: {e}; \
             cannot proceed with an epic operation against unknown state"
        )
    })?;
    if let Some(epic) = select.select(|s| product_management::epic::get(&s.epics, epic_id)) {
        return Ok(epic);
    }
    if backing_story_via_select(select, id).is_some() {
        return Err(not_an_epic_message(id));
    }
    Err(format!(
        "epic #{id} not found in canonical store after refresh_story \
         round-trip; it may not exist at source-of-truth"
    ))
}

fn not_an_epic_message(id: u64) -> String {
    format!("#{id} is not an Epic (story_type mismatch); only Epic stories can be parents")
}
