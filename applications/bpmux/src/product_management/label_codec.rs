use crate::product_management::iteration::label::IterationLabelParts;
use crate::product_management::story::{Acceptor, Tag};

pub trait LabelCodec {
    fn decode_iteration(&self, label: &str) -> Option<IterationLabelParts>;

    fn is_iteration_label(&self, label: &str) -> bool {
        self.decode_iteration(label).is_some()
    }

    fn is_iteration_label_loose(&self, label: &str) -> bool;

    fn encode_iteration(&self, name: &str, position: u32) -> String;

    fn iteration_label_class(&self, label: &str) -> Option<String>;

    fn encode_parent_epic(&self, issue_number: u64) -> String;

    fn encode_tag(&self, tag: Tag) -> &'static str;

    fn encode_acceptor(&self, acceptor: Acceptor) -> &'static str;
}
