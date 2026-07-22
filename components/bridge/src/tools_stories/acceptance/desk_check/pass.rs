use crate::tools_stories::acceptance::helpers::{
    BugGate, format_bypass_payload, recent_evidence_for_story, verify_bug_question_or_bypass,
};
use crate::tools_stories::event_meta::meta_for;
use crate::tools_stories::helpers::ensure_story_in_canonical_store;
use product_management::story::StoryId;

fn parse_acceptor_arg(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<product_management::story::Acceptor, String> {
    let raw = args.get("acceptor").and_then(|v| v.as_str()).ok_or_else(|| {
        serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "pass-desk-check",
            "missing_evidence": ["acceptor-class"],
            "hint": "pass_desk_check requires acceptor: \"team\" | \"owner\" | \"dev\" — designer voices acceptor-class at gate-fire per owner-canon 2026-05-20 03:08:51 UTC + feedback_designer_voices_acceptor_class_at_pass_desk_check; \"dev\" routes [Chore]-class to dev-acceptance (chore-ownership-triad)",
        })
        .to_string()
    })?;
    product_management::story::Acceptor::parse(raw.trim()).ok_or_else(|| {
        serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "pass-desk-check",
            "missing_evidence": ["acceptor-class"],
            "observed": raw,
            "allowed": ["team", "owner", "dev"],
        })
        .to_string()
    })
}

pub async fn handle_pass_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    role: &str,
    actor_id: Option<&str>,
    refresh: Option<&std::sync::Arc<sync::SyncOrchestrator>>,
    select: &store::Select,
    hub: Option<&std::sync::Arc<hub::Hub>>,
) -> Result<String, String> {
    let actor_role = role;
    let workflow = workflow::current_workflow();
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let acceptor = parse_acceptor_arg(args)?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::PassDeskCheck)?;
    let story_opt = refresh.and_then(|_| {
        select
            .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(id)))
    });
    let class = story_opt
        .as_ref()
        .map(|story| product_management::story::gate_class_from_acceptor(story.acceptor()))
        .unwrap_or(workflow::GateClass::TeamVerifiable);
    let transition = workflow::resolve_transition(workflow, "pass-desk-check", class)
        .map_err(|e| e.as_json_string())?;
    workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_name().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if hub.is_some() && refresh.is_some() {
        let evidence = recent_evidence_for_story(select, id);
        workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
            None,
        )
        .map_err(|e| e.as_json_string())?;
    }
    let ci_snapshot = select
        .select(|state| store::ci::snapshot(&state.ci))
        .map(|snapshot| workflow::CiSnapshot {
            conclusion: snapshot.conclusion.to_string(),
            head_sha: snapshot.head_sha.unwrap_or_default(),
        });
    workflow::check_ci_green(transition, ci_snapshot.as_ref()).map_err(|e| e.as_json_string())?;
    let was_closed = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, select, id).await?;
        if let Some(body) = prior.body.as_deref() {
            crate::atp_substantive_gate::verify_atp_substantive(body)?;
        }
        prior.is_concluded()
    } else {
        false
    };
    if let Some(r) = refresh {
        let meta = meta_for(agent_name, actor_id);
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
                "#{id} pass-desk-check has no forward state from '{}'",
                transition.from
            )
        })?;
        let mut events = Vec::new();
        if was_closed {
            events.push(product_management::story::events::StoryEvent::new(
                meta.clone(),
                StoryId::new(id),
                product_management::story::events::StoryEventKind::StoryReopened,
            ));
        }
        events.push(product_management::story::events::StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            product_management::story::events::StoryEventKind::AcceptorAssigned { acceptor },
        ));
        events.push(product_management::story::events::StoryEvent::new(
            meta.clone(),
            StoryId::new(id),
            product_management::story::events::StoryEventKind::VerdictCast {
                verdict: product_management::story::Verdict::Succeed {
                    step: "desk-check".into(),
                },
                payload: bypass.clone().map(format_bypass_payload),
            },
        ));
        events.push(product_management::story::events::StoryEvent::new(
            meta,
            StoryId::new(id),
            product_management::story::events::StoryEventKind::StateTransitioned {
                new_state,
                placement,
            },
        ));
        r.mutate_batch(events)
            .map_err(|e| format!("#{id} pass-desk-check enqueue failed: {e}"))?;
        return Ok(format!(
            "#{id} passed desk-check: signal sent, label applied"
        ));
    }
    Err("pass_desk_check requires refresh — tracker fallback retired in slice 5n".into())
}
