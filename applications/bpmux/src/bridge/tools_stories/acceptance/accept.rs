use super::helpers::{
    BugGate, format_bypass_payload, gate_to_id, recent_evidence_for_story,
    verify_bug_question_or_bypass,
};
use crate::bridge::tools_stories::build_verdict_reprompt_signal;
use crate::bridge::tools_stories::helpers::{ensure_story_in_canonical_store, role_from_name};
use crate::product_management::story::StoryId;

pub(crate) async fn handle_accept_story(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::sync::RefreshDriver>>,
    session: Option<&std::sync::Arc<crate::session::Session>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::Accept)?;
    let r = refresh.ok_or(
        "accept_story requires refresh (RefreshDriver) — tracker fallback retired in slice 5n",
    )?;
    r.test_race_window_rendezvous().await;
    let story_opt = r.story(id);
    let class = story_opt
        .as_ref()
        .map(|s| {
            crate::product_management::story::gate_class_from_acceptor_and_story_type(
                s.acceptor(),
                s.story_type(),
            )
        })
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "accept", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if let Some(s) = session {
        let evidence = recent_evidence_for_story(s, id)?;
        let current_head = crate::bridge::tools::git_repo::current_head_sha();
        if let Err(e) = crate::workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
            current_head.as_deref(),
        ) {
            for role in e.expired_evidence_roles() {
                if let crate::transport::protocol::ClientMessage::Send {
                    text,
                    to,
                    addressee,
                    message_type,
                } = build_verdict_reprompt_signal(id, &role)
                {
                    let reprompt = crate::message::Message {
                        from: "bpmux".to_string(),
                        text,
                        ts: chrono::Utc::now(),
                        message_type: crate::message::MessageType::from(message_type.as_deref()),
                        to,
                        addressee,
                        explicit_privacy: true,
                        ..crate::message::Message::default()
                    };
                    if let Err(err) = s.broadcast(&reprompt) {
                        crate::diag::log_diag(&format!(
                            "bridge: verdict-reprompt broadcast failed: {err}"
                        ));
                    }
                }
            }
            return Err(e.as_json_string());
        }
    }
    if let Some(story) = story_opt.as_ref() {
        crate::workflow::check_target_state_valid(transition, story.state.as_str())
            .map_err(|e| e.as_json_string())?;
        if let Some(comments) = story.gate_comments.as_deref() {
            let prior_fires: Vec<crate::workflow::PriorFire> = comments
                .iter()
                .map(|gc| crate::workflow::PriorFire {
                    gate_id: gate_to_id(gc.gate).to_string(),
                    actor_role: gc.actor.clone(),
                })
                .collect();
            crate::workflow::check_self_mutation_refused(
                actor_role,
                "accept",
                &prior_fires,
                &["deliver", "pass-desk-check"],
            )
            .map_err(|e| e.as_json_string())?;
        }
    }
    let actor = crate::routing::short_name(agent_name).to_string();
    accept_via_canonical_store(r, id, actor, bypass).await?;
    r.test_race_window_complete();
    Ok(format!("#{id} accepted and closed"))
}

pub(crate) async fn accept_via_canonical_store(
    refresh: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    id: u64,
    actor: String,
    bypass: Option<String>,
) -> Result<(), String> {
    ensure_story_in_canonical_store(refresh, id).await?;
    let workflow = crate::workflow::current_workflow();
    let transition = workflow
        .transition_by_id("accept")
        .ok_or("workflow has no 'accept' transition")?;
    let (new_state, placement) =
        crate::board::advance(&transition.from, &crate::board::Verdict::Succeed, workflow)
            .ok_or_else(|| {
                format!(
                    "#{id} accept has no forward state from '{}'",
                    transition.from
                )
            })?;
    let meta = crate::product_management::story::events::EventMeta {
        actor,
        ts: chrono::Utc::now(),
        peer_id: crate::product_management::story::peer::PeerId::local(),
    };
    refresh
        .mutate_sync(
            crate::product_management::story::events::StoryEvent::GateCommentAppended {
                meta: meta.clone(),
                id: StoryId::new(id),
                gate: crate::product_management::story::Gate::Accepted,
                payload: bypass.map(format_bypass_payload),
            },
        )
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
    refresh
        .mutate_sync(
            crate::product_management::story::events::StoryEvent::StateTransitioned {
                meta: meta.clone(),
                id: StoryId::new(id),
                new_state,
                placement,
            },
        )
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
    refresh
        .mutate_sync(
            crate::product_management::story::events::StoryEvent::StoryClosed {
                meta,
                id: StoryId::new(id),
                reason: None,
            },
        )
        .map_err(|e| format!("#{id} close enqueue failed: {e}"))?;
    refresh
        .drain_pending()
        .await
        .map_err(|e| format!("#{id} drain after accept failed: {e}"))?;
    Ok(())
}
