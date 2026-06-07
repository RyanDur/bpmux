use crate::board::domain::{Gate, GateComment};

pub fn enrich_body(original_body: &str, context: &str) -> String {
    format!("{context}\n{original_body}")
}

pub fn latest_reject(gates: &[GateComment]) -> Option<&GateComment> {
    gates
        .iter()
        .rev()
        .find(|g| matches!(g.gate, Gate::Rejected | Gate::FailDeskCheck))
}

pub fn reject_gate_label(gate: Gate) -> &'static str {
    match gate {
        Gate::FailDeskCheck => "fail-desk-check",
        _ => "rejected",
    }
}

pub fn reject_lead(gc: &GateComment) -> Option<String> {
    let reason = gc.payload.as_deref()?;
    Some(format!(
        "⚠ REWORK — {} by {} ({}):\n\n{reason}\n\n---\n",
        reject_gate_label(gc.gate),
        gc.actor,
        gc.ts.to_rfc3339()
    ))
}
