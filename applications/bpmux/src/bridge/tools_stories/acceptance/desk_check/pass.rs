use crate::bridge::tools_stories::acceptance::helpers::{
    BugGate, format_bypass_payload, recent_evidence_for_story, verify_bug_question_or_bypass,
};
use crate::bridge::tools_stories::helpers::{
    LABEL_AWAITING_DEV, LABEL_AWAITING_OWNER, LABEL_AWAITING_TEAM, LABEL_DELIVERED,
    ensure_story_in_canonical_store, reap_workflow_labels, role_from_name,
};

const ALL_ACCEPTOR_LABELS: [&str; 3] = [
    LABEL_AWAITING_TEAM,
    LABEL_AWAITING_OWNER,
    LABEL_AWAITING_DEV,
];

#[derive(Clone, Copy)]
enum AcceptorClass {
    Team,
    Owner,
    Dev,
}

impl AcceptorClass {
    fn label(self) -> &'static str {
        match self {
            AcceptorClass::Team => LABEL_AWAITING_TEAM,
            AcceptorClass::Owner => LABEL_AWAITING_OWNER,
            AcceptorClass::Dev => LABEL_AWAITING_DEV,
        }
    }

    fn stale_labels(self) -> impl Iterator<Item = &'static str> {
        let keep = self.label();
        ALL_ACCEPTOR_LABELS.into_iter().filter(move |l| *l != keep)
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "team" => Some(AcceptorClass::Team),
            "owner" => Some(AcceptorClass::Owner),
            "dev" => Some(AcceptorClass::Dev),
            _ => None,
        }
    }
}

fn parse_acceptor_arg(
    args: &serde_json::Map<String, serde_json::Value>,
) -> Result<AcceptorClass, String> {
    let raw = args.get("acceptor").and_then(|v| v.as_str()).ok_or_else(|| {
        serde_json::json!({
            "kind": "workflow-gate-mismatch",
            "attempted_transition": "pass-desk-check",
            "missing_evidence": ["acceptor-class"],
            "hint": "pass_desk_check requires acceptor: \"team\" | \"owner\" | \"dev\" — designer voices acceptor-class at gate-fire per owner-canon 2026-05-20 03:08:51 UTC + feedback_designer_voices_acceptor_class_at_pass_desk_check; \"dev\" routes [Chore]-class to dev-acceptance (chore-ownership-triad)",
        })
        .to_string()
    })?;
    AcceptorClass::parse(raw.trim()).ok_or_else(|| {
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
    refresh: Option<&std::sync::Arc<dyn crate::board::RefreshDriver>>,
    session: Option<&std::sync::Arc<crate::session::Session>>,
) -> Result<String, String> {
    let actor_role = role_from_name(agent_name);
    let workflow = crate::workflow::current_workflow();
    let number = args
        .get("number")
        .and_then(|v| v.as_u64())
        .ok_or("missing 'number' argument")?;
    let acceptor = parse_acceptor_arg(args)?;
    let bypass = verify_bug_question_or_bypass(args, BugGate::PassDeskCheck)?;
    let r_opt = refresh;
    let story_opt = r_opt.and_then(|r| r.store().story(number));
    let class = story_opt
        .as_ref()
        .map(|s| crate::workflow::gate_class_from_labels(&s.raw_labels))
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
        let evidence = recent_evidence_for_story(s, number)?;
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
        let prior = ensure_story_in_canonical_store(r, number).await?;
        if let Some(body) = prior.body.as_deref() {
            crate::bridge::atp_substantive_gate::verify_atp_substantive(body)?;
        }
        prior.is_closed
    } else {
        false
    };
    if let Some(r) = refresh {
        let meta = crate::board::events::EventMeta {
            actor: crate::transport::ws::short_name(agent_name).to_string(),
            ts: chrono::Utc::now(),
            peer_id: crate::board::PeerId::local(),
        };
        if was_closed {
            r.mutate_sync(crate::board::BoardEvent::StoryReopened {
                meta: meta.clone(),
                number,
            })
            .map_err(|e| format!("#{number} reopen enqueue failed: {e}"))?;
        }
        reap_workflow_labels(r.as_ref(), number, &meta, LABEL_DELIVERED)?;
        let labels_after_reap = r.story(number).map(|s| s.raw_labels).unwrap_or_default();
        for stale in acceptor.stale_labels() {
            if labels_after_reap.iter().any(|l| l == stale) {
                r.mutate_sync(crate::board::BoardEvent::TagRemoved {
                    meta: meta.clone(),
                    number,
                    label: stale.to_string(),
                })
                .map_err(|e| format!("#{number} {stale}-remove enqueue failed: {e}"))?;
            }
        }
        let acceptor_label = acceptor.label();
        if !labels_after_reap.iter().any(|l| l == acceptor_label) {
            r.mutate_sync(crate::board::BoardEvent::TagAdded {
                meta: meta.clone(),
                number,
                label: acceptor_label.to_string(),
            })
            .map_err(|e| format!("#{number} {acceptor_label}-add enqueue failed: {e}"))?;
        }
        r.mutate_sync(crate::board::BoardEvent::GateCommentAppended {
            meta: meta.clone(),
            number,
            gate: crate::board::domain::Gate::PassDeskCheck,
            payload: bypass.clone().map(format_bypass_payload),
        })
        .map_err(|e| format!("#{number} gate enqueue failed: {e}"))?;
        r.mutate_sync(crate::board::BoardEvent::StateTransitioned {
            meta,
            number,
            new_state: crate::board::domain::StoryState::Delivered,
        })
        .map_err(|e| format!("#{number} state enqueue failed: {e}"))?;
        r.drain_pending()
            .await
            .map_err(|e| format!("#{number} drain after pass_desk_check failed: {e}"))?;
        return Ok(format!(
            "#{number} passed desk-check: signal sent, label applied"
        ));
    }
    Err(
        "pass_desk_check requires refresh (RefreshDriver) — tracker fallback retired in slice 5n"
            .into(),
    )
}
