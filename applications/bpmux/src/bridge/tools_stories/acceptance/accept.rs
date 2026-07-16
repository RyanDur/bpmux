use super::helpers::{
    BugGate, format_bypass_payload, recent_evidence_for_story, verdict_to_id,
    verify_bug_question_or_bypass,
};
use crate::bridge::tools_stories::build_verdict_reprompt_signal;
use crate::bridge::tools_stories::event_meta::meta_for;
use crate::bridge::tools_stories::helpers::ensure_story_in_canonical_store;
use product_management::story::StoryId;

pub(crate) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    select: &crate::store::Select,
    hub: Option<&std::sync::Arc<crate::hub::Hub>>,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::Accept)?;
    let r = refresh.ok_or(
        "accept_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    r.test_race_window_rendezvous().await;
    let story_opt = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(id)));
    let class = story_opt
        .as_ref()
        .map(|s| {
            product_management::story::gate_class_from_acceptor_and_story_type(
                s.acceptor(),
                s.story_type(),
            )
        })
        .unwrap_or(workflow::GateClass::TeamVerifiable);
    let transition =
        workflow::resolve_transition(workflow, "accept", class).map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if let Some(s) = hub {
        let evidence = recent_evidence_for_story(r, id);
        let current_head = crate::bridge::tools::git_repo::current_head_sha();
        if let Err(e) = workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
            current_head.as_deref(),
        ) {
            for role in e.expired_evidence_roles() {
                if let protocol::ClientMessage::Send {
                    text,
                    to,
                    addressee,
                    message_type,
                } = build_verdict_reprompt_signal(id, &role)
                {
                    let reprompt = message::Message {
                        from: "bpmux".to_string(),
                        text,
                        ts: chrono::Utc::now(),
                        message_type: message::MessageType::from(message_type.as_deref()),
                        to,
                        addressee,
                        explicit_privacy: true,
                        ..message::Message::default()
                    };
                    if let Err(err) = s.broadcast(&reprompt) {
                        diag::log_diag(&format!(
                            "bridge: verdict-reprompt broadcast failed: {err}"
                        ));
                    }
                }
            }
            return Err(e.as_json_string());
        }
    }
    if let Some(story) = story_opt.as_ref() {
        workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
        if let Some(comments) = story.verdicts.as_deref() {
            let prior_fires: Vec<workflow::PriorFire> = comments
                .iter()
                .map(|gc| workflow::PriorFire {
                    gate_id: verdict_to_id(&gc.verdict).to_string(),
                    actor_role: gc.actor.clone(),
                })
                .collect();
            workflow::check_self_mutation_refused(
                actor_role,
                "accept",
                &prior_fires,
                &["deliver", "pass-desk-check"],
            )
            .map_err(|e| e.as_json_string())?;
        }
    }
    accept_via_canonical_store(r, select, id, agent_name, actor_id, bypass).await?;
    r.test_race_window_complete();
    Ok(format!("#{id} accepted and closed"))
}

pub(crate) async fn accept_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    select: &crate::store::Select,
    id: u64,
    agent_name: &str,
    actor_id: Option<&str>,
    bypass: Option<String>,
) -> Result<(), String> {
    ensure_story_in_canonical_store(refresh, select, id).await?;
    let workflow = workflow::current_workflow();
    let transition = workflow
        .transition_by_id("accept")
        .ok_or("workflow has no 'accept' transition")?;
    let board::Decision {
        coarse: new_state,
        placement,
    } = board::advance(
        &product_management::story::Verdict::Succeed {
            step: transition.from.as_str().into(),
        },
        workflow,
    )
    .ok_or_else(|| {
        format!(
            "#{id} accept has no forward state from '{}'",
            transition.from
        )
    })?;
    let meta = meta_for(agent_name, actor_id);
    refresh
        .mutate_batch_sync(vec![
            product_management::story::events::StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                product_management::story::events::StoryEventKind::VerdictCast {
                    verdict: product_management::story::Verdict::Succeed {
                        step: "acceptance".into(),
                    },
                    payload: bypass.map(format_bypass_payload),
                },
            ),
            product_management::story::events::StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                product_management::story::events::StoryEventKind::StateTransitioned {
                    new_state,
                    placement,
                },
            ),
            product_management::story::events::StoryEvent::new(
                meta,
                StoryId::new(id),
                product_management::story::events::StoryEventKind::StoryClosed { reason: None },
            ),
        ])
        .map_err(|e| format!("#{id} accept enqueue failed: {e}"))?;
    Ok(())
}
