pub fn story_type_from_labels(
    labels: &[String],
) -> Option<crate::product_management::story::StoryKind> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .decode_story_type(l)
    })
}

pub fn parent_epic_from_labels(labels: &[String]) -> Option<u64> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .decode_parent_epic(l)
    })
}

pub fn outcome_membership_from_labels(labels: &[String]) -> Option<(u64, u32)> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .decode_parent_outcome(l)
    })
}

pub fn iteration_order_from_labels(labels: &[String]) -> Option<u32> {
    labels
        .iter()
        .filter_map(|l| {
            crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
                .decode_iteration_order(l)
        })
        .min()
}

pub fn ready_order_from_labels(labels: &[String]) -> Option<u32> {
    use crate::product_management::board::label_codec::LabelCodec;
    labels
        .iter()
        .filter_map(|l| {
            crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
                .decode_ready_order(l)
        })
        .min()
}

pub fn iteration_review_closers_from_labels(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|l| {
            crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
                .decode_iteration_review_closer(l)
        })
        .collect()
}

pub fn iteration_dev_ok_by_from_labels(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|l| {
            crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
                .decode_iteration_dev_ok(l)
        })
        .collect()
}

pub fn is_current_from_labels(labels: &[String]) -> bool {
    labels.iter().any(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .is_current_iteration_label(l)
    })
}

pub fn acceptor_from_labels(
    labels: &[String],
) -> Option<crate::product_management::story::Acceptor> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec.decode_acceptor(l)
    })
}

pub fn blocked_by_from_labels(labels: &[String]) -> Option<u64> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .decode_blocked_by(l)
    })
}

pub fn horizon_from_labels(labels: &[String]) -> Option<crate::product_management::story::Horizon> {
    labels.iter().find_map(|l| {
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec.decode_horizon(l)
    })
}

pub fn horizon_order_from_labels(labels: &[String]) -> Option<u32> {
    labels
        .iter()
        .filter_map(|l| {
            crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
                .decode_horizon_order(l)
        })
        .min()
}

fn is_iteration_position_label(label: &str) -> bool {
    crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
        .is_iteration_position_label(label)
}

pub(crate) fn is_position_label(label: &str) -> bool {
    use crate::product_management::board::label_codec::LabelCodec;
    is_iteration_position_label(label)
        || crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .is_ready_order_label(label)
}

#[cfg(test)]
pub(crate) fn is_iteration_position_label_for_test(label: &str) -> bool {
    is_iteration_position_label(label)
}

#[cfg(test)]
pub(crate) fn is_position_label_for_test(label: &str) -> bool {
    is_position_label(label)
}
