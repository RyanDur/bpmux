use serde_json::Map;
use serde_json::Value;

use crate::product_management::story::{BoardState, Stage};

const TARGET_STATE_RETIRED: &str = "retired";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum TargetState {
    Workflow(BoardState),
    Retired,
}

impl TargetState {
    pub(super) fn parse(s: &str) -> Result<Self, String> {
        let normalized = s.to_ascii_lowercase();
        if normalized == TARGET_STATE_RETIRED {
            return Ok(Self::Retired);
        }
        BoardState::parse(&normalized).map(Self::Workflow).ok_or_else(|| {
            format!(
                "unknown target_state '{s}'. Valid: icebox, ready, in-progress, desk-check, delivered, accepted, retired"
            )
        })
    }

    pub(super) fn as_label(&self) -> Option<&'static str> {
        match self {
            Self::Workflow(BoardState::Icebox) => None,
            Self::Workflow(state) => Some(state.github_label()),
            Self::Retired => Some(BoardState::Retired.github_label()),
        }
    }

    pub(super) fn closes_issue(&self) -> bool {
        matches!(self, Self::Retired | Self::Workflow(BoardState::Accepted))
    }

    pub(super) fn as_display(&self) -> &'static str {
        match self {
            Self::Workflow(state) => state.as_str(),
            Self::Retired => BoardState::Retired.as_str(),
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
    story: Option<&crate::product_management::story::Story>,
    fine: Option<Stage>,
) -> &'static str {
    match story {
        Some(s) if s.is_closed => "retired-or-accepted",
        Some(s) => match &s.state {
            BoardState::Icebox => "icebox",
            BoardState::Ready => "ready",
            BoardState::InProgress => match fine {
                Some(Stage::DeskCheck) => "desk-check",
                _ => "in-progress",
            },
            BoardState::Acceptance => "acceptance",
            BoardState::Accepted => "accepted",
            BoardState::Retired => "retired",
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
