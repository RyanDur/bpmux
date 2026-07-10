use super::super::config::Transition;
use super::GateError;

#[derive(Debug, Clone)]
pub struct CiSnapshot {
    pub conclusion: String,
    pub head_sha: String,
}

pub fn check_ci_green(
    transition: &Transition,
    snapshot: Option<&CiSnapshot>,
) -> Result<(), GateError> {
    let Some(snap) = snapshot else {
        return Err(GateError::CiNotGreen {
            attempted_transition: transition.id.clone(),
            ci_conclusion: "unknown".to_string(),
            head_sha: String::new(),
            green_head_check_path: "neither".to_string(),
        });
    };
    match snap.conclusion.as_str() {
        "success" => Ok(()),
        "quota_exhausted" => Ok(()),
        other => Err(GateError::CiNotGreen {
            attempted_transition: transition.id.clone(),
            ci_conclusion: other.to_string(),
            head_sha: snap.head_sha.clone(),
            green_head_check_path: "neither".to_string(),
        }),
    }
}
