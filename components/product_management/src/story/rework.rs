use super::CastVerdict;

pub fn most_recent_needs_work(verdicts: &[CastVerdict]) -> Option<&CastVerdict> {
    verdicts
        .iter()
        .max_by_key(|v| v.ts)
        .filter(|v| v.verdict.is_fail())
}
