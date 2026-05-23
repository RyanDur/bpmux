use tokio::sync::Mutex;

use super::super::tools::WsSink;
use super::helpers::{
    compose_iteration_body, compose_story_body, require_nonempty_string, role_from_name,
};
use super::signals::broadcast_board_invalidate;

pub(crate) async fn handle_create_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let workflow = crate::workflow::current_workflow();
    let pm = workflow.role_id_for_purpose("pm").unwrap_or("pm");
    let dev = workflow.role_id_for_purpose("dev").unwrap_or("dev");
    let actor_role = role_from_name(agent_name);
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("missing 'title' argument")?;
    let is_chore = crate::board::domain::story_type_from_labels_or_title(&[], title)
        == crate::board::domain::StoryType::Chore;
    if actor_role != pm && !(actor_role == dev && is_chore) {
        return Err(
            "create_story is pm-lane (dev may author [Chore]-class stories per chore-ownership \
             canon). If you have a pain to file, surface it in broadcast chat and pm will file \
             the story."
                .to_string(),
        );
    }
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
    if crate::board::domain::story_type_from_labels_or_title(&[], title)
        != crate::board::domain::StoryType::Chore
    {
        crate::bridge::atp_substantive_gate::verify_atp_pre_filing(
            &composed_body,
            acceptance_test_path,
        )
        .map_err(|e| e.as_json_string())?;
    }
    let parent_epic = args.get("parent_epic").and_then(|v| v.as_u64());
    if let Some(r) = refresh {
        if let Some(epic_number) = parent_epic {
            let parent = r.story(epic_number).ok_or_else(|| {
                format!(
                    "epic #{epic_number} not found in canonical store — wait for next ingest tick or call refresh_story explicitly"
                )
            })?;
            if parent.story_type != crate::board::domain::StoryType::Epic {
                return Err(format!(
                    "story #{epic_number} is not an Epic (story_type={:?}); only Epic stories can be parents",
                    parent.story_type
                ));
            }
        }
        let number = r
            .create_story(title.to_string(), composed_body.clone())
            .await
            .map_err(|e| format!("create_story failed: {e}"))?;
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        let story_type = crate::board::domain::story_type_from_labels_or_title(&[], title);
        r.mutate_sync(crate::board::BoardEvent::StoryCreated {
            meta: meta.clone(),
            number,
            title: title.to_string(),
            body: composed_body,
            initial_state: crate::board::domain::StoryState::Icebox,
            story_type,
        })
        .map_err(|e| format!("#{number} created but enqueue failed: {e}"))?;
        if let Some(epic_number) = parent_epic {
            r.mutate_sync(crate::board::BoardEvent::ParentEpicSet {
                meta,
                number,
                epic_number,
            })
            .map_err(|e| format!("#{number} parent_epic enqueue failed: {e}"))?;
        }
        broadcast_board_invalidate(ws_tx).await;
        return match parent_epic {
            Some(epic_number) => Ok(format!(
                "story created: #{number} {title} (under epic #{epic_number})"
            )),
            None => Ok(format!("story created: #{number} {title}")),
        };
    }
    Err(
        "create_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}

pub(crate) async fn handle_create_iteration(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    ws_tx: &Mutex<Option<WsSink>>,
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
) -> Result<String, String> {
    let pm = crate::workflow::current_workflow()
        .role_id_for_purpose("pm")
        .unwrap_or("pm");
    if role_from_name(agent_name) != pm {
        return Err(
            "create_iteration is pm-lane. Iteration entities are PM-owned per Epic #1189."
                .to_string(),
        );
    }
    let theme_name = require_nonempty_string(
        args,
        "theme_name",
        "theme_name is required: short name for the iteration (e.g., 'Aquila', 'Pegasus'). Surfaces as the GH issue title 'Iteration {theme_name}'.",
    )?;
    let r = refresh
        .ok_or("create_iteration requires refresh (RefreshDriver) — canonical store unavailable")?;
    let existing_open_iteration = r.all_stories().into_iter().find(|s| {
        s.story_type == crate::board::domain::StoryType::Iteration
            && !s.is_closed
            && s.title
                .eq_ignore_ascii_case(&format!("Iteration {theme_name}"))
    });
    if let Some(existing) = existing_open_iteration {
        return Err(format!(
            "iteration named '{theme_name}' already open as #{n}; close it first or pick a different name (#1281 sister-discipline: duplicate-collision rejected at create-time)",
            n = existing.number
        ));
    }
    let title = format!("Iteration {theme_name}");
    let body = compose_iteration_body(theme_name);
    let number = r
        .create_story(title.clone(), body.clone())
        .await
        .map_err(|e| format!("create_iteration adapter call failed: {e}"))?;
    let meta = crate::board::events::EventMeta {
        actor: crate::transport::ws::short_name(agent_name).to_string(),
        ts: chrono::Utc::now(),
        peer_id: crate::board::PeerId::local(),
    };
    r.mutate_sync(crate::board::BoardEvent::StoryCreated {
        meta: meta.clone(),
        number,
        title: title.clone(),
        body: body.clone(),
        initial_state: crate::board::domain::StoryState::Icebox,
        story_type: crate::board::domain::StoryType::Iteration,
    })
    .map_err(|e| format!("#{number} iteration created but enqueue failed: {e}"))?;
    r.mutate_sync(crate::board::BoardEvent::TagAdded {
        meta,
        number,
        label: "iteration".to_string(),
    })
    .map_err(|e| format!("#{number} iteration label enqueue failed: {e}"))?;
    broadcast_board_invalidate(ws_tx).await;
    Ok(format!("iteration created: #{number} {title}"))
}
