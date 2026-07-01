use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::iteration::label::IterationLabelParts;

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
            name: name.to_string(),
            position,
        })
    }

    fn is_iteration_label_loose(&self, label: &str) -> bool {
        label.starts_with("iteration-") && label.contains('.')
    }

    fn encode_iteration(&self, name: &str, position: u32) -> String {
        format!("iteration-{name}.{position}")
    }

    fn iteration_label_class(&self, label: &str) -> Option<String> {
        let parts = self.decode_iteration(label)?;
        Some(format!("iteration-{}.", parts.name))
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

    pub fn encode_parent_iteration(&self, iteration_number: u64, position: u32) -> String {
        format!("parent-iteration-#{iteration_number}.{position}")
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

    pub fn decode_horizon(&self, label: &str) -> Option<crate::product_management::story::Horizon> {
        use crate::product_management::story::Horizon;
        match label.strip_prefix("horizon.")? {
            "now" => Some(Horizon::Now),
            "next" => Some(Horizon::Next),
            "later" => Some(Horizon::Later),
            _ => None,
        }
    }

    pub fn encode_horizon(&self, horizon: crate::product_management::story::Horizon) -> String {
        format!("horizon.{}", horizon.as_str())
    }

    pub fn encode_horizon_order(&self, position: u32) -> String {
        format!("horizon-order.{position}")
    }

    pub fn decode_horizon_order(&self, label: &str) -> Option<u32> {
        label.strip_prefix("horizon-order.")?.parse().ok()
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

    pub fn encode_current_iteration(&self) -> String {
        CURRENT_ITERATION_LABEL.to_string()
    }

    pub fn decode_acceptor(
        &self,
        label: &str,
    ) -> Option<crate::product_management::story::Acceptor> {
        use crate::product_management::story::Acceptor;
        match label {
            ACCEPTOR_TEAM_LABEL => Some(Acceptor::Team),
            ACCEPTOR_OWNER_LABEL => Some(Acceptor::Owner),
            ACCEPTOR_DEV_LABEL => Some(Acceptor::Dev),
            _ => None,
        }
    }

    pub fn encode_acceptor(
        &self,
        acceptor: crate::product_management::story::Acceptor,
    ) -> &'static str {
        use crate::product_management::story::Acceptor;
        match acceptor {
            Acceptor::Team => ACCEPTOR_TEAM_LABEL,
            Acceptor::Owner => ACCEPTOR_OWNER_LABEL,
            Acceptor::Dev => ACCEPTOR_DEV_LABEL,
        }
    }

    pub fn decode_tag(&self, label: &str) -> Option<crate::product_management::story::Tag> {
        use crate::product_management::story::Tag;
        if label.eq_ignore_ascii_case(BLOCKED_LABEL) {
            Some(Tag::Blocked)
        } else {
            None
        }
    }

    pub fn encode_tag(&self, tag: crate::product_management::story::Tag) -> &'static str {
        use crate::product_management::story::Tag;
        match tag {
            Tag::Blocked => BLOCKED_LABEL,
        }
    }

    pub fn decode_story_type(
        &self,
        label: &str,
    ) -> Option<crate::product_management::story::StoryKind> {
        use crate::product_management::story::StoryKind;
        match label.to_ascii_lowercase().as_str() {
            "feature" => Some(StoryKind::Feature),
            "bug" => Some(StoryKind::Bug),
            "chore" => Some(StoryKind::Chore),
            _ => None,
        }
    }

    pub fn encode_story_type(
        &self,
        story_type: crate::product_management::story::StoryKind,
    ) -> String {
        story_type.as_label().to_string()
    }
}

pub const CURRENT_ITERATION_LABEL: &str = "current-iteration";
pub const BLOCKED_LABEL: &str = "blocked";
pub const ACCEPTOR_TEAM_LABEL: &str = "awaiting team";
pub const ACCEPTOR_OWNER_LABEL: &str = "awaiting owner";
pub const ACCEPTOR_DEV_LABEL: &str = "awaiting dev";
