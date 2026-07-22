use super::event_meta::meta_for;
use super::helpers::ensure_story_in_canonical_store;
use product_management::story::StoryId;

const REASON_CITE_MIN_CHARS: usize = 16;

pub async fn handle_close_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<sync::SyncOrchestrator>>,
    select: &store::Select,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let transition = workflow::resolve_transition(workflow, "close", workflow::GateClass::None)
        .map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let reason = args
        .get("reason")
        .and_then(|v| v.as_str())
        .ok_or("missing 'reason' argument")?;
    let disposition = args
        .get("disposition")
        .and_then(|v| v.as_str())
        .unwrap_or("retire");
    let terminal_state = match disposition {
        "retire" | "wontfix" => product_management::story::Work::Retired,
        "accept" => product_management::story::Work::Accepted,
        other => {
            return Err(format!(
                "unknown disposition '{other}' — expected one of: retire, wontfix, accept"
            ));
        }
    };
    let trimmed_reason = reason.trim();
    if trimmed_reason.len() < REASON_CITE_MIN_CHARS {
        let err = serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "close",
            "missing_evidence": ["reason-cite"],
            "min_chars": REASON_CITE_MIN_CHARS,
            "observed_chars": trimmed_reason.len(),
        });
        return Err(err.to_string());
    }
    if let Some(r) = refresh {
        ensure_story_in_canonical_store(r, select, id).await?;
        let meta = meta_for(agent_name, actor_id);
        if matches!(terminal_state, product_management::story::Work::Retired) {
            r.mutate(product_management::story::events::StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                product_management::story::events::StoryEventKind::TagAdded {
                    label: "retired".to_string(),
                },
            ))
            .map_err(|e| format!("#{id} retired-tag enqueue failed: {e}"))?;
        }
        r.mutate(product_management::story::events::StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            product_management::story::events::StoryEventKind::StateTransitioned {
                new_state: terminal_state,
                placement: None,
            },
        ))
        .map_err(|e| format!("#{id} close-state enqueue failed: {e}"))?;
        r.mutate(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(id),
            product_management::story::events::StoryEventKind::StoryClosed {
                reason: Some(reason.to_string()),
            },
        ))
        .map_err(|e| format!("#{id} close enqueue failed: {e}"))?;
        return Ok(format!("#{id} closed: {reason}"));
    }
    Err("close_story requires refresh — tracker fallback retired in slice 5n".into())
}
