use product_management::story::{CastVerdict, Verdict};

pub fn enrich_body(original_body: &str, context: &str) -> String {
    format!("{context}\n{original_body}")
}

pub fn latest_reject(gates: &[CastVerdict]) -> Option<&CastVerdict> {
    gates.iter().rev().find(|g| g.verdict.is_fail())
}

pub fn reject_label(verdict: &Verdict) -> &'static str {
    match verdict {
        Verdict::Fail { step } if step.as_str() == "desk-check" => "fail-desk-check",
        _ => "rejected",
    }
}

pub fn reject_lead(gc: &CastVerdict) -> Option<String> {
    let reason = gc.payload.as_deref()?;
    Some(format!(
        "⚠ REWORK — {} by {} ({}):\n\n{reason}\n\n---\n",
        reject_label(&gc.verdict),
        gc.actor,
        gc.ts.to_rfc3339()
    ))
}
