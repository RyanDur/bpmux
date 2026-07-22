use product_management::iteration::Iteration;
use product_management::scheduling::ProjectedState;
use product_management::story::Story;

pub async fn handle_list_ready(
    select: &store::Select,
    iterations: &[Iteration],
) -> Result<String, String> {
    let current_order = product_management::iteration::label::current_iteration(iterations)
        .map(|iteration| iteration.stories.clone())
        .unwrap_or_default();
    let ready: Vec<Story> = select
        .select(|state| product_management::story::slice::all(&state.stories))
        .into_iter()
        .filter(|story| {
            product_management::scheduling::projected_state(story, iterations)
                == ProjectedState::Ready
        })
        .collect();
    let mut queue: Vec<&Story> = current_order
        .iter()
        .filter_map(|id| ready.iter().find(|s| s.id == *id))
        .collect();
    let mut loose: Vec<&Story> = ready
        .iter()
        .filter(|s| !current_order.contains(&s.id))
        .collect();
    loose.sort_by_key(|s| s.id);
    queue.extend(loose);
    let rows: Vec<serde_json::Value> = queue
        .iter()
        .enumerate()
        .map(|(idx, s)| {
            serde_json::json!({
                "position": idx + 1,
                "id": s.id,
                "title": s.title,
            })
        })
        .collect();
    serde_json::to_string_pretty(&rows).map_err(|e| format!("serialize: {e}"))
}
