use workflow::EvidenceMessage;

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

pub(crate) fn verdict_to_id(verdict: &product_management::story::Verdict) -> &'static str {
    use product_management::story::Verdict;
    match verdict {
        Verdict::Succeed { step } => match step.as_str() {
            "ready" => "pickup",
            "in-progress" => "deliver",
            "desk-check" => "pass-desk-check",
            _ => "accept",
        },
        Verdict::Fail { step } => match step.as_str() {
            "desk-check" => "fail-desk-check",
            _ => "reject",
        },
    }
}

pub(crate) fn recent_evidence_for_story(
    select: &crate::store::Select,
    story_id: u64,
) -> Vec<EvidenceMessage> {
    select
        .select(|state| {
            product_management::evidence::evidence_for_story(
                &state.evidence,
                product_management::story::StoryId::new(story_id),
            )
        })
        .into_iter()
        .map(|evidence| EvidenceMessage {
            from_role: evidence.from_role,
            ts: evidence.ts,
            grounded_head: evidence.grounded_head,
        })
        .collect()
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
