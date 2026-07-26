use serde_json::Map;
use serde_json::Value;

use product_management::story::{Stage, Work};

const TARGET_STATE_RETIRED: &str = "retired";
const TARGET_STATE_ICEBOX: &str = "icebox";
const TARGET_STATE_READY: &str = "ready";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TargetState {
    Icebox,
    Ready,
    Workflow(Work),
    Retired,
}

impl TargetState {
    pub(super) fn parse(s: &str) -> Result<Self, String> {
        let normalized = s.to_ascii_lowercase();
        if normalized == TARGET_STATE_ICEBOX {
            return Ok(Self::Icebox);
        }
        if normalized == TARGET_STATE_READY {
            return Ok(Self::Ready);
        }
        if normalized == TARGET_STATE_RETIRED {
            return Ok(Self::Retired);
        }
        if let Some(state) = Work::parse(&normalized)
            && state != Work::Ready
        {
            return Ok(Self::Workflow(state));
        }
        let middle = workflow::board_middle_stage_ids(workflow::current_workflow());
        if middle.iter().any(|stage| stage.as_str() == normalized) {
            return Ok(Self::Workflow(Work::InProgress));
        }
        let stages: Vec<&str> = middle.iter().map(|stage| stage.as_str()).collect();
        Err(format!(
            "unknown target_state '{s}'. Valid: icebox, ready, {}, acceptance, accepted, retired",
            stages.join(", ")
        ))
    }

    pub(super) fn workflow_state(&self) -> Option<Work> {
        match self {
            Self::Icebox | Self::Ready => Some(Work::Ready),
            Self::Workflow(state) => Some(state.clone()),
            Self::Retired => None,
        }
    }

    pub(super) fn as_label(&self) -> Option<&'static str> {
        match self {
            Self::Icebox => None,
            Self::Ready => {
                Some(product_management::scheduling::ProjectedState::Ready.github_label())
            }
            Self::Workflow(state) => state.github_label(),
            Self::Retired => Work::Retired.github_label(),
        }
    }

    pub(super) fn closes_issue(&self) -> bool {
        matches!(self, Self::Retired | Self::Workflow(Work::Accepted))
    }

    pub(super) fn as_display(&self) -> &'static str {
        match self {
            Self::Icebox => TARGET_STATE_ICEBOX,
            Self::Ready => TARGET_STATE_READY,
            Self::Workflow(state) => state.as_str(),
            Self::Retired => Work::Retired.as_str(),
        }
    }
}

pub(super) fn parse_position(args: &Map<String, Value>) -> Result<Option<usize>, String> {
    match args.get("position") {
        None | Some(Value::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_u64()
                .ok_or("'position' must be a 1-based integer or null")?;
            if n == 0 {
                return Err("'position' is 1-based; got 0".to_string());
            }
            Ok(Some(n as usize))
        }
    }
}

pub(super) fn require_nonempty(
    args: &Map<String, Value>,
    key: &str,
    hint: &str,
) -> Result<String, String> {
    let value = args
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing '{key}' argument; {hint}"))?
        .trim()
        .to_string();
    if value.is_empty() {
        return Err(format!("'{key}' must not be blank; {hint}"));
    }
    Ok(value)
}

pub(super) fn current_state_display(
    story: Option<&product_management::story::Story>,
    fine: Option<Stage>,
    iterations: &[product_management::iteration::Iteration],
) -> &'static str {
    use product_management::scheduling::projected_state;
    match story {
        Some(s) if s.is_concluded() => "retired-or-accepted",
        Some(s) => match &s.state {
            Work::Ready => projected_state(s, iterations).as_str(),
            Work::InProgress => match fine {
                Some(Stage::DeskCheck) => "desk-check",
                _ => "in-progress",
            },
            Work::Acceptance => "acceptance",
            Work::Accepted => "accepted",
            Work::Retired => "retired",
        },
        None => "unknown",
    }
}

pub(super) fn audit_comment_body(
    actor_short: &str,
    from_state: &str,
    target: &TargetState,
    position: Option<usize>,
    rationale: &str,
) -> String {
    let pos_part = match position {
        Some(p) => format!(" position={p}"),
        None => String::new(),
    };
    format!(
        "**move_story** by {actor_short}: {from_state} → {to}{pos_part}\n\n{rationale}",
        to = target.as_display(),
    )
}
