use crate::product_management::iteration::label::IterationLabelParts;

pub trait LabelCodec {
    fn decode_ready_order(&self, label: &str) -> Option<u32>;

    fn is_ready_order_label(&self, label: &str) -> bool {
        self.decode_ready_order(label).is_some()
    }

    fn encode_ready_order(&self, position: u32) -> String;

    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts>;

    fn is_iteration_label(&self, label: &str) -> bool {
        self.decode_iteration(label).is_some()
    }

    fn is_iteration_label_loose(&self, label: &str) -> bool;

    fn encode_iteration(&self, name: &str, position: u32) -> String;

    fn iteration_label_class(&self, label: &str) -> Option<String>;

    fn encode_parent_epic(&self, issue_number: u64) -> String;
}
