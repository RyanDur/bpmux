use super::CastVerdict;

pub fn most_recent_needs_work(verdicts: &[CastVerdict]) -> Option<&CastVerdict> {
    verdicts.last().filter(|v| v.verdict.is_fail())
}
