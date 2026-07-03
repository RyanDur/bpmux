use super::{Gate, GateComment};

pub fn most_recent_needs_work(gates: &[GateComment]) -> Option<&GateComment> {
    gates
        .last()
        .filter(|g| matches!(g.gate, Gate::Rejected | Gate::FailDeskCheck))
}
