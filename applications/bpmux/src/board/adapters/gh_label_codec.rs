use crate::board::iteration_label::IterationLabelParts;
use crate::board::label_codec::LabelCodec;

pub struct GhLabelCodec;

impl LabelCodec for GhLabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("ready-order.")?.parse().ok()
    }

    fn encode_ready_order(&self, position: u32) -> String {
        format!("ready-order.{position}")
    }

    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts> {
        let rest = label.strip_prefix("iteration-")?;
        let dot_idx = rest.rfind('.')?;
        let (name, dotted_pos) = rest.split_at(dot_idx);
        let position = dotted_pos.trim_start_matches('.').parse::<u32>().ok()?;
        if name.is_empty() {
            return None;
        }
        Some(IterationLabelParts {
            theme_name: name.to_string(),
            position,
        })
    }

    fn is_iteration_label_loose(&self, label: &str) -> bool {
        label.starts_with("iteration-") && label.contains('.')
    }

    fn encode_iteration(&self, theme_name: &str, position: u32) -> String {
        format!("iteration-{theme_name}.{position}")
    }

    fn iteration_label_class(&self, label: &str) -> Option<String> {
        let parts = self.decode_iteration(label)?;
        Some(format!("iteration-{}.", parts.theme_name))
    }

    fn encode_parent_epic(&self, issue_number: u64) -> String {
        format!("parent-epic-#{issue_number}")
    }
}

impl GhLabelCodec {
    pub fn decode_parent_epic(&self, label: &str) -> Option<u64> {
        label.strip_prefix("parent-epic-#")?.parse().ok()
    }

    pub fn decode_parent_iteration(&self, label: &str) -> Option<(u64, u32)> {
        let rest = label.strip_prefix("parent-iteration-#")?;
        let (number, position) = rest.split_once('.')?;
        Some((number.parse().ok()?, position.parse().ok()?))
    }

    pub fn decode_parent_outcome(&self, label: &str) -> Option<(u64, u32)> {
        let rest = label.strip_prefix("parent-outcome-#")?;
        let (number, position) = rest.split_once('.')?;
        Some((number.parse().ok()?, position.parse().ok()?))
    }

    pub fn decode_iteration_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("iteration-order.")?.parse().ok()
    }

    pub fn encode_iteration_order(&self, position: u32) -> String {
        format!("iteration-order.{position}")
    }

    pub fn decode_iteration_review_closer(&self, label: &str) -> Option<String> {
        let role = label.strip_prefix("iteration-review-closed.")?;
        if role.is_empty() {
            return None;
        }
        Some(role.to_string())
    }

    pub fn encode_iteration_review_closed(&self, role: &str) -> String {
        format!("iteration-review-closed.{role}")
    }

    pub fn decode_iteration_dev_ok(&self, label: &str) -> Option<String> {
        let role = label.strip_prefix("iteration-dev-ok.")?;
        if role.is_empty() {
            return None;
        }
        Some(role.to_string())
    }

    pub fn encode_iteration_dev_ok(&self, role: &str) -> String {
        format!("iteration-dev-ok.{role}")
    }

    pub fn decode_horizon(&self, label: &str) -> Option<crate::board::domain::Horizon> {
        use crate::board::domain::Horizon;
        match label.strip_prefix("horizon.")? {
            "now" => Some(Horizon::Now),
            "next" => Some(Horizon::Next),
            "later" => Some(Horizon::Later),
            _ => None,
        }
    }

    pub fn encode_horizon(&self, horizon: crate::board::domain::Horizon) -> String {
        format!("horizon.{}", horizon.as_str())
    }

    pub fn is_iteration_position_label(&self, label: &str) -> bool {
        let Some(rest) = label.strip_prefix("iteration-") else {
            return false;
        };
        if rest.starts_with("order.") {
            return false;
        }
        let Some(dot_idx) = rest.rfind('.') else {
            return false;
        };
        rest.get(dot_idx + 1..)
            .is_some_and(|tail| !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()))
    }

    pub fn is_current_iteration_label(&self, label: &str) -> bool {
        label == CURRENT_ITERATION_LABEL
    }

    pub fn decode_acceptor(&self, label: &str) -> Option<crate::board::domain::Acceptor> {
        use crate::board::domain::Acceptor;
        match label {
            ACCEPTOR_TEAM_LABEL => Some(Acceptor::Team),
            ACCEPTOR_OWNER_LABEL => Some(Acceptor::Owner),
            ACCEPTOR_DEV_LABEL => Some(Acceptor::Dev),
            _ => None,
        }
    }
}

pub const CURRENT_ITERATION_LABEL: &str = "current-iteration";
pub const ACCEPTOR_TEAM_LABEL: &str = "awaiting team";
pub const ACCEPTOR_OWNER_LABEL: &str = "awaiting owner";
pub const ACCEPTOR_DEV_LABEL: &str = "awaiting dev";
