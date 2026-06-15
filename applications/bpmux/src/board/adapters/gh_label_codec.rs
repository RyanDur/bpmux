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

#[cfg(test)]
mod tests {
    use super::GhLabelCodec;
    use crate::board::domain::Acceptor;

    #[test]
    fn decode_acceptor_maps_each_awaiting_label_1551() {
        assert_eq!(
            GhLabelCodec.decode_acceptor("awaiting team"),
            Some(Acceptor::Team)
        );
        assert_eq!(
            GhLabelCodec.decode_acceptor("awaiting owner"),
            Some(Acceptor::Owner)
        );
        assert_eq!(
            GhLabelCodec.decode_acceptor("awaiting dev"),
            Some(Acceptor::Dev)
        );
        assert_eq!(GhLabelCodec.decode_acceptor("feature"), None);
        assert_eq!(GhLabelCodec.decode_acceptor("blocked"), None);
    }

    #[test]
    fn decode_parent_epic_parses_issue_number() {
        assert_eq!(
            GhLabelCodec.decode_parent_epic("parent-epic-#1477"),
            Some(1477)
        );
    }

    #[test]
    fn decode_parent_epic_rejects_non_epic_labels() {
        assert_eq!(GhLabelCodec.decode_parent_epic("iteration-Octans.3"), None);
        assert_eq!(GhLabelCodec.decode_parent_epic("parent-epic-#x"), None);
    }

    #[test]
    fn decode_parent_iteration_parses_number_and_position() {
        assert_eq!(
            GhLabelCodec.decode_parent_iteration("parent-iteration-#1495.7"),
            Some((1495, 7))
        );
    }

    #[test]
    fn decode_parent_iteration_rejects_malformed() {
        assert_eq!(
            GhLabelCodec.decode_parent_iteration("parent-iteration-#1495"),
            None
        );
        assert_eq!(
            GhLabelCodec.decode_parent_iteration("parent-epic-#1495"),
            None
        );
    }

    #[test]
    fn iteration_review_closed_round_trips_role() {
        let encoded = GhLabelCodec.encode_iteration_review_closed("designer");
        assert_eq!(encoded, "iteration-review-closed.designer");
        assert_eq!(
            GhLabelCodec.decode_iteration_review_closer(&encoded),
            Some("designer".to_string())
        );
    }

    #[test]
    fn decode_iteration_review_closer_rejects_other_labels() {
        assert_eq!(
            GhLabelCodec.decode_iteration_review_closer("iteration-order.3"),
            None
        );
        assert_eq!(
            GhLabelCodec.decode_iteration_review_closer("iteration-review-closed."),
            None
        );
        assert_eq!(GhLabelCodec.decode_iteration_review_closer("feature"), None);
    }

    #[test]
    fn iteration_dev_ok_round_trips_role() {
        let encoded = GhLabelCodec.encode_iteration_dev_ok("dev");
        assert_eq!(encoded, "iteration-dev-ok.dev");
        assert_eq!(
            GhLabelCodec.decode_iteration_dev_ok(&encoded),
            Some("dev".to_string())
        );
    }

    #[test]
    fn decode_iteration_dev_ok_rejects_other_labels() {
        assert_eq!(
            GhLabelCodec.decode_iteration_dev_ok("iteration-order.3"),
            None
        );
        assert_eq!(
            GhLabelCodec.decode_iteration_dev_ok("iteration-dev-ok."),
            None
        );
        assert_eq!(
            GhLabelCodec.decode_iteration_dev_ok("iteration-review-closed.designer"),
            None
        );
    }

    #[test]
    fn horizon_round_trips_each_bucket() {
        use crate::board::domain::Horizon;
        for (h, label) in [
            (Horizon::Now, "horizon.now"),
            (Horizon::Next, "horizon.next"),
            (Horizon::Later, "horizon.later"),
        ] {
            assert_eq!(GhLabelCodec.encode_horizon(h), label);
            assert_eq!(GhLabelCodec.decode_horizon(label), Some(h));
        }
    }

    #[test]
    fn decode_horizon_rejects_other_labels() {
        assert_eq!(GhLabelCodec.decode_horizon("horizon.someday"), None);
        assert_eq!(GhLabelCodec.decode_horizon("iteration-order.3"), None);
        assert_eq!(GhLabelCodec.decode_horizon("feature"), None);
    }

    #[test]
    fn decode_iteration_order_parses_position() {
        assert_eq!(
            GhLabelCodec.decode_iteration_order("iteration-order.300"),
            Some(300)
        );
        assert_eq!(
            GhLabelCodec.decode_iteration_order("iteration-order.1"),
            Some(1)
        );
    }

    #[test]
    fn decode_iteration_order_rejects_non_order_labels() {
        assert_eq!(
            GhLabelCodec.decode_iteration_order("iteration-Octans.3"),
            None
        );
        assert_eq!(
            GhLabelCodec.decode_iteration_order("iteration-order.x"),
            None
        );
        assert_eq!(GhLabelCodec.decode_iteration_order("ready-order.5"), None);
        assert_eq!(
            GhLabelCodec.decode_iteration_order("parent-iteration-#9.3"),
            None
        );
    }

    #[test]
    fn is_iteration_position_label_accepts_theme_dot_digits() {
        assert!(GhLabelCodec.is_iteration_position_label("iteration-Octans.3"));
    }

    #[test]
    fn is_iteration_position_label_excludes_order_and_non_digit_tails() {
        assert!(!GhLabelCodec.is_iteration_position_label("iteration-order.5"));
        assert!(!GhLabelCodec.is_iteration_position_label("iteration-Octans.x"));
        assert!(!GhLabelCodec.is_iteration_position_label("ready-order.5"));
    }
}
