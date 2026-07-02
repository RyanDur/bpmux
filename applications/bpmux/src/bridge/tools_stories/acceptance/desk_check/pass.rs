use crate::bridge::tools_stories::acceptance::helpers::{
    BugGate, format_bypass_payload, recent_evidence_for_story, verify_bug_question_or_bypass,
};
use crate::bridge::tools_stories::helpers::{ensure_story_in_canonical_store, role_from_name};
use crate::product_management::story::StoryId;

fn parse_acceptor_arg(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<crate::product_management::story::Acceptor, String> {
    let raw = args.get("acceptor").and_then(|v| v.as_str()).ok_or_else(|| {
        serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "pass-desk-check",
            "missing_evidence": ["acceptor-class"],
            "hint": "pass_desk_check requires acceptor: \"team\" | \"owner\" | \"dev\" — designer voices acceptor-class at gate-fire per owner-canon 2026-05-20 03:08:51 UTC + feedback_designer_voices_acceptor_class_at_pass_desk_check; \"dev\" routes [Chore]-class to dev-acceptance (chore-ownership-triad)",
        })
        .to_string()
    })?;
    crate::product_management::story::Acceptor::parse(raw.trim()).ok_or_else(|| {
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

pub(crate) async fn handle_pass_desk_check(
    args: &serde_json::Map<String, serde_json::Value>,
    agent_name: &str,
    refresh: Option<&std::sync::Arc<dyn crate::product_management::board::RefreshDriver>>,
    session: Option<&std::sync::Arc<crate::session::Session>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let id = args
        .get("id")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'id' argument")?;
    let acceptor = parse_acceptor_arg(args)?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::PassDeskCheck)?;
    let r_opt = refresh;
    let story_opt = r_opt.and_then(|r| r.story(id));
    let class = story_opt
        .as_ref()
        .map(|s| crate::workflow::gate_class_from_acceptor(s.acceptor()))
        .unwrap_or(crate::workflow::GateClass::TeamVerifiable);
    let transition = crate::workflow::resolve_transition(workflow, "pass-desk-check", class)
        .map_err(|e| e.as_json_string())?;
    crate::workflow::enforce_role_allowed(
        transition,
        actor_role,
        workflow.owner_role_id().unwrap_or(""),
    )
    .map_err(|e| e.as_json_string())?;
    if let Some(s) = session {
        let evidence = recent_evidence_for_story(s, id)?;
        crate::workflow::check_evidence_present(
            transition,
            actor_role,
            &evidence,
            chrono::Utc::now(),
            None,
        )
        .map_err(|e| e.as_json_string())?;
    }
    let ci_snapshot = refresh.and_then(|r| r.ci_state());
    crate::workflow::check_ci_green(transition, ci_snapshot.as_ref())
        .map_err(|e| e.as_json_string())?;
    let was_closed = if let Some(r) = refresh {
        let prior = ensure_story_in_canonical_store(r, id).await?;
        if let Some(body) = prior.body.as_deref() {
            crate::bridge::atp_substantive_gate::verify_atp_substantive(body)?;
        }
        prior.is_closed
    } else {
        false
    };
    if let Some(r) = refresh {
        let meta = crate::product_management::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::product_management::board::PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(
                crate::product_management::board::StoryEvent::StoryReopened {
                    meta: meta.clone(),
                    id: StoryId::new(id),
                },
            )
            .map_err(|e| format!("#{id} reopen enqueue failed: {e}"))?;
        }
        r.mutate_sync(
            crate::product_management::board::StoryEvent::AcceptorAssigned {
                meta: meta.clone(),
                id: StoryId::new(id),
                acceptor,
            },
        )
        .map_err(|e| format!("#{id} acceptor enqueue failed: {e}"))?;
        r.mutate_sync(
            crate::product_management::board::StoryEvent::GateCommentAppended {
                meta: meta.clone(),
                id: StoryId::new(id),
                gate: crate::product_management::story::Gate::PassDeskCheck,
                payload: bypass.clone().map(format_bypass_payload),
            },
        )
        .map_err(|e| format!("#{id} gate enqueue failed: {e}"))?;
        r.mutate_sync(
            crate::product_management::board::StoryEvent::StateTransitioned {
                meta,
                id: StoryId::new(id),
                new_state: crate::product_management::story::BoardState::Acceptance,
            },
        )
        .map_err(|e| format!("#{id} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{id} drain after pass_desk_check failed: {e}"))?;
        return Ok(format!(
            "#{id} passed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "pass_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
