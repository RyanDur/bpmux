use serde_json::Map;
use serde_json::Value;

use crate::board::domain::StoryState;

const TARGET_STATE_RETIRED: &str = "retired";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TargetState {
    Workflow(StoryState),
    Retired,
}

impl TargetState {
    pub(super) fn parse(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "icebox" => Ok(Self::Workflow(StoryState::Icebox)),
            "ready" => Ok(Self::Workflow(StoryState::Ready)),
            "in-progress" | "in_progress" | "in progress" => {
                Ok(Self::Workflow(StoryState::InProgress))
            }
            "desk-check" | "desk_check" | "deskcheck" => Ok(Self::Workflow(StoryState::DeskCheck)),
            "delivered" => Ok(Self::Workflow(StoryState::Delivered)),
            "accepted" => Ok(Self::Workflow(StoryState::Accepted)),
            TARGET_STATE_RETIRED => Ok(Self::Retired),
            other => Err(format!(
                "unknown target_state '{other}'. Valid: icebox, ready, in-progress, desk-check, delivered, accepted, retired"
            )),
        }
    }

    pub(super) fn as_label(self) -> Option<&'static str> {
        match self {
            Self::Workflow(StoryState::Icebox) => None,
            Self::Workflow(state) => Some(state.github_label()),
            Self::Retired => Some(StoryState::Retired.github_label()),
        }
    }

    pub(super) fn closes_issue(self) -> bool {
        matches!(self, Self::Retired | Self::Workflow(StoryState::Accepted))
    }

    pub(super) fn as_display(self) -> &'static str {
        match self {
            Self::Workflow(state) => state.as_str(),
            Self::Retired => StoryState::Retired.as_str(),
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

pub(super) fn current_state_display(story: Option<&crate::board::domain::Story>) -> &'static str {
    match story {
        Some(s) if s.is_closed => "retired-or-accepted",
        Some(s) => match s.state {
            StoryState::Icebox => "icebox",
            StoryState::Ready => "ready",
            StoryState::InProgress => "in-progress",
            StoryState::DeskCheck => "desk-check",
            StoryState::Delivered => "delivered",
            StoryState::Acceptance => "acceptance",
            StoryState::Accepted => "accepted",
            StoryState::Retired => "retired",
        },
        None => "unknown",
    }
}

pub(super) fn audit_comment_body(
    actor_short: &str,
    from_state: &str,
    target: TargetState,
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
