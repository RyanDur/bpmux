use super::{CURRENT_ITERATION_LABEL, GhLabelCodec};

impl GhLabelCodec {
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
}
