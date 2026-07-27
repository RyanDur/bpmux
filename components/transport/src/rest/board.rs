pub fn ordered_board_stories(select: &store::Select) -> Vec<serde_json::Value> {
    select.select(board_stories_json)
}

pub(crate) fn board_stories_json(state: &store::AppState) -> Vec<serde_json::Value> {
    let all = product_management::story::slice::all(&state.stories);
    let iterations = product_management::iteration::all(&state.iterations);
    let codec = source_codec::gh_label_codec::GhLabelCodec;
    let items: Vec<serde_json::Value> = all
        .iter()
        .map(|story| {
            let projected = product_management::scheduling::projected_state(story, &iterations);
            let wire =
                board::wire_state(&projected, board::held_stage(story, &state.board).as_ref());
            view::board::story_to_board_json(&codec, story, &wire)
        })
        .collect();
    let current_order: Vec<u64> =
        product_management::iteration::label::current_iteration(&iterations)
            .map(|iteration| iteration.stories.iter().map(|id| id.value()).collect())
            .unwrap_or_default();
    sort_ready_by_membership(items, &current_order)
}

pub fn sort_ready_by_membership(
    stories: Vec<serde_json::Value>,
    current_order: &[u64],
) -> Vec<serde_json::Value> {
    let mut by_state: Vec<(String, Vec<serde_json::Value>)> = Vec::new();
    for s in stories {
        let state = s
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        match by_state.iter_mut().find(|(k, _)| k == &state) {
            Some((_, list)) => list.push(s),
            None => by_state.push((state, vec![s])),
        }
    }
    for (state, list) in by_state.iter_mut() {
        if state == product_management::scheduling::ProjectedState::Ready.as_str() {
            list.sort_by_cached_key(|s| {
                let id = s.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
                product_management::scheduling::ready_membership_rank(id, current_order)
            });
        }
    }
    by_state.into_iter().flat_map(|(_, list)| list).collect()
}

pub fn board_stories(select: &store::Select) -> Vec<serde_json::Value> {
    ordered_board_stories(select)
}

pub fn board_columns_json() -> Vec<serde_json::Value> {
    let workflow = workflow::current_workflow();
    let middle = workflow::board_middle_stage_ids(workflow);
    board::board_columns(&middle)
        .iter()
        .map(|column| {
            let id = column_wire_id(column);
            serde_json::json!({ "id": id, "label": title_case(&id) })
        })
        .collect()
}

fn column_wire_id(column: &board::Column) -> String {
    use board::Column;
    match column {
        Column::Icebox => "icebox".to_string(),
        Column::Ready => "ready".to_string(),
        Column::Middle(stage) => stage.as_str().to_string(),
        Column::Acceptance => "acceptance".to_string(),
        Column::Accepted => "accepted".to_string(),
        Column::Retired => "retired".to_string(),
    }
}

fn title_case(id: &str) -> String {
    id.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ")
}

pub fn board_epics(select: &store::Select) -> Vec<serde_json::Value> {
    select.select(|state| {
        let stories = product_management::story::slice::all(&state.stories);
        let iterations = product_management::iteration::all(&state.iterations);
        product_management::epic::all(&state.epics)
            .iter()
            .map(|epic| {
                let children = product_management::epic::children_of(&stories, epic.id);
                let derived = product_management::epic::derive_epic_state(&children, &iterations);
                serde_json::json!({
                    "id": epic.id.value(),
                    "title": epic.title,
                    "state": derived.as_str(),
                    "closed": epic.is_closed,
                    "labels": ["epic"],
                    "assignees": [],
                    "created_at": epic.filed_at.to_rfc3339(),
                    "body": epic.body.clone().unwrap_or_default(),
                    "parent_epic": serde_json::Value::Null,
                })
            })
            .collect()
    })
}

pub fn board_iterations(select: &store::Select) -> Vec<serde_json::Value> {
    use view::board::IterationView;
    select.select(|state| {
        let outcomes = product_management::outcome::all(&state.outcomes);
        let workflow = workflow::current_workflow();
        product_management::iteration::all(&state.iterations)
            .iter()
            .map(|iteration| {
                serde_json::to_value(IterationView::project(iteration, &outcomes, workflow))
                    .unwrap_or_default()
            })
            .collect()
    })
}

pub fn board_roadmap(select: &store::Select) -> Vec<serde_json::Value> {
    select.select(|state| {
        product_management::roadmap::selectors::roadmap_cards(
            &product_management::outcome::all(&state.outcomes),
            &product_management::iteration::all(&state.iterations),
            workflow::current_workflow(),
        )
        .iter()
        .filter_map(|card| serde_json::to_value(card).ok())
        .collect()
    })
}

pub fn board_outcomes(select: &store::Select) -> Vec<serde_json::Value> {
    use view::board::OutcomeView;
    select.select(|state| {
        let all_iterations = product_management::iteration::all(&state.iterations);
        let workflow = workflow::current_workflow();
        product_management::outcome::all(&state.outcomes)
            .iter()
            .filter(|outcome| !outcome.is_retired())
            .map(|outcome| {
                serde_json::to_value(OutcomeView::project(outcome, &all_iterations, workflow))
                    .unwrap_or_default()
            })
            .collect()
    })
}
