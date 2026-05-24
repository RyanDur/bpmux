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
}

#[cfg(test)]
mod tests {
    use super::GhLabelCodec;

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
