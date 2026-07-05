use crate::workflow::EvidenceMessage;

#[derive(Clone, Copy)]
pub(crate) enum BugGate {
    Accept,
    PassDeskCheck,
}

impl BugGate {
    fn name(self) -> &'static str {
        match self {
            BugGate::Accept => "accept",
            BugGate::PassDeskCheck => "pass desk-check",
        }
    }

    fn alt_gate(self) -> &'static str {
        match self {
            BugGate::Accept => "reject_story",
            BugGate::PassDeskCheck => "fail_desk_check",
        }
    }

    fn past_tense(self) -> &'static str {
        match self {
            BugGate::Accept => "accepted",
            BugGate::PassDeskCheck => "passed",
        }
    }
}

pub(crate) fn gate_to_id(gate: crate::product_management::story::Gate) -> &'static str {
    match gate {
        crate::product_management::story::Gate::Filed => "filed",
        crate::product_management::story::Gate::Pickup => "pickup",
        crate::product_management::story::Gate::Delivered => "deliver",
        crate::product_management::story::Gate::PassDeskCheck => "pass-desk-check",
        crate::product_management::story::Gate::FailDeskCheck => "fail-desk-check",
        crate::product_management::story::Gate::Rejected => "reject",
        crate::product_management::story::Gate::Accepted => "accept",
    }
}

pub(crate) fn recent_evidence_for_story(
    session: &std::sync::Arc<crate::session::Session>,
    story_id: u64,
) -> Result<Vec<EvidenceMessage>, String> {
    let store = crate::product_management::verdict::load(&session.dir);
    let verdicts = crate::product_management::verdict::verdicts_for_story(
        &store,
        crate::product_management::story::StoryId::new(story_id),
    );
    Ok(verdicts
        .into_iter()
        .map(|v| EvidenceMessage {
            from_role: v.from_role,
            ts: v.ts,
            grounded_head: v.grounded_head,
        })
        .collect())
}

pub(crate) fn verify_bug_question_or_bypass(
    args: &serde_json::Map<String, serde_json::Value>,
    gate: BugGate,
) -> Result<Option<String>, String> {
    let would_file_bug = args.get("would_file_bug").and_then(|v| v.as_bool()).ok_or_else(|| {
        format!(
            "{} requires 'would_file_bug: bool' argument. Bug-question test: would you file a bug about this if you {} it as-is? If yes -> use {} instead (per `feedback_bug_question_test_at_desk_check`).",
            gate.name(),
            gate.past_tense(),
            gate.alt_gate(),
        )
    })?;
    if !would_file_bug {
        return Ok(None);
    }
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    if !force {
        return Err(format!(
            "Cannot {}: would_file_bug=true. Use {} instead -- accept-and-file-bug-as-deferral is the anti-pattern this gate closes (per `feedback_bug_question_test_at_desk_check`). To override with explicit justification, pass force=true and bypass_reason: \"<reason>\".",
            gate.name(),
            gate.alt_gate(),
        ));
    }
    let reason = args
        .get("bypass_reason")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "{} bypass requires 'bypass_reason: String' (non-empty) -- recorded in gate-comment audit trail per `feedback_external_catch_over_self_catch`.",
                gate.name(),
            )
        })?;
    Ok(Some(reason.to_string()))
}

pub(crate) fn format_bypass_payload(reason: String) -> String {
    format!("[bug-question bypass] {reason}")
}
