use super::helpers::{compose_story_body, require_nonempty_string, role_from_name};
use crate::product_management::epic::EpicId;
use crate::product_management::story::StoryId;

pub(crate) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    dispatch: &crate::store::Dispatch,
) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let story_type = crate::product_management::story::story_type_from_title(title);
    validate_actor_role_can_create(agent_name, story_type)?;
    let user_job = require_nonempty_string(
        args,
        "user_job",
        "user_job is required (2026-04-22 retro): 'When <situation>, <role> needs <affordance> because <constraint/goal>.' Cannot file a story without articulating the user-job it answers.",
    )?;
    let observation_falsifier = require_nonempty_string(
        args,
        "observation_falsifier",
        "observation_falsifier is required (2026-04-22 retro): 'I would reject this story if I observed X.' Frames acceptance as observation-that-fails rather than mechanism-that-passes.",
    )?;
    let acceptance_test_path = args
        .get("acceptance_test_path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let author_body = args.get("body").and_then(|v| v.as_str()).unwrap_or("");
    let composed_body = compose_story_body(
        user_job,
        observation_falsifier,
        acceptance_test_path,
        author_body,
    );
    if story_type != crate::product_management::story::StoryKind::Chore {
        crate::bridge::atp_substantive_gate::verify_atp_pre_filing(
            &composed_body,
            acceptance_test_path,
        )
        .map_err(|e| e.as_json_string())?;
    }
    let parent_epic = args
        .get("parent_epic")
        .and_then(|v| v.as_u64())
        .map(EpicId::new);
    let r = refresh.ok_or(
        "create_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    validate_parent_epic(r, parent_epic)?;
    let acceptor = match args.get("acceptor").and_then(|v| v.as_str()) {
        Some(raw) => Some(
            crate::product_management::story::Acceptor::parse(raw.trim()).ok_or_else(|| {
                format!("acceptor must be one of: team | owner | dev (got {raw:?})")
            })?,
        ),
        None => super::acceptor_axis::derive_acceptor(user_job),
    };
    let epic_body = composed_body.clone();
    let id = create_and_broadcast_story_events(
        r,
        agent_name,
        title,
        composed_body,
        story_type,
        parent_epic,
        acceptor,
    )
    .await?;
    if crate::product_management::board::adapters::github::is_epic_title(title) {
        dispatch.dispatch(crate::store::AppAction::Epic(
            crate::product_management::epic::EpicAction::Upserted(vec![
                crate::product_management::epic::Epic {
                    id: crate::product_management::epic::EpicId::new(id),
                    title: title.to_string(),
                    body: Some(epic_body),
                    filed_at: chrono::Utc::now(),
                    filed_by: crate::transport::ws::short_name(agent_name).to_string(),
                    state: crate::product_management::story::BoardState::Icebox,
                    is_closed: false,
                },
            ]),
        ));
        r.store().remove(StoryId::new(id));
    }
    Ok(match parent_epic {
        Some(epic) => {
            format!("story created: #{id} {title} (under epic {epic})")
        }
        None => format!("story created: #{id} {title}"),
    })
}

fn validate_actor_role_can_create(
    agent_name: &str,
    story_type: crate::product_management::story::StoryKind,
) -> Result<(), String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let dev = workflow.role_id_for_purpose("dev").unwrap_or("dev");
    let actor_role = role_from_name(agent_name);
    if let Some(owner) = workflow.owner_role_id()
        && !owner.is_empty()
        && actor_role == owner
    {
        return Ok(());
    }
    let is_chore = story_type == crate::product_management::story::StoryKind::Chore;
    if actor_role != pm && !(actor_role == dev && is_chore) {
        return Err(
            "create_story is pm-lane (dev may author [Chore]-class stories per chore-ownership \
             canon). If you have a pain to file, surface it in broadcast chat and pm will file \
             the story."
                .to_string(),
        );
    }
    Ok(())
}

fn validate_parent_epic(
    r: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    parent_epic: Option<EpicId>,
) -> Result<(), String> {
    let Some(epic) = parent_epic else {
        return Ok(());
    };
    if r.epic(epic).is_some() {
        return Ok(());
    }
    if let Some(parent) = r.story(epic.value()) {
        return Err(format!(
            "story {epic} is not an Epic (story_type={:?}); only Epic stories can be parents",
            parent.story_type()
        ));
    }
    Err(format!(
        "epic {epic} not found in canonical store — wait for next ingest tick or call refresh_story explicitly"
    ))
}

async fn create_and_broadcast_story_events(
    r: &std::sync::Arc<dyn crate::product_management::board::RefreshDriver>,
    agent_name: &str,
    title: &str,
    composed_body: String,
    story_type: crate::product_management::story::StoryKind,
    parent_epic: Option<EpicId>,
    acceptor: Option<crate::product_management::story::Acceptor>,
) -> Result<u64, String> {
    let id = r
        .create_story(title.to_string(), composed_body.clone(), Vec::new())
        .await
        .map_err(|e| format!("create_story failed: {e}"))?;
    let meta = crate::product_management::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::board::PeerId::local(),
    };
    r.mutate_sync(crate::product_management::board::StoryEvent::StoryCreated {
        meta: meta.clone(),
        id: StoryId::new(id),
        title: title.to_string(),
        body: composed_body,
        initial_state: crate::product_management::story::BoardState::Icebox,
        story_type,
    })
    .map_err(|e| format!("#{id} created but enqueue failed: {e}"))?;
    if let Some(acceptor) = acceptor {
        r.mutate_sync(
            crate::product_management::board::StoryEvent::AcceptorAssigned {
                meta: meta.clone(),
                id: StoryId::new(id),
                acceptor,
            },
        )
        .map_err(|e| format!("#{id} acceptor enqueue failed: {e}"))?;
    }
    if let Some(epic_id) = parent_epic {
        r.mutate_sync(
            crate::product_management::board::StoryEvent::ParentEpicSet {
                meta,
                id: StoryId::new(id),
                epic_id,
            },
        )
        .map_err(|e| format!("#{id} parent_epic enqueue failed: {e}"))?;
    }
    Ok(id)
}
