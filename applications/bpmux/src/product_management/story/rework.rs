use super::GateComment;

pub fn most_recent_needs_work(gates: &[GateComment]) -> Option<&GateComment> {
    gates.last().filter(|g| g.gate.is_fail())
}
