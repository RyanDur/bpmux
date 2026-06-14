#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

mod stale_workflow_labels_filter {
    use super::*;

    #[test]
    fn empty_current_labels_returns_no_stale() {
        let result = stale_workflow_labels(&[], "ready");
        assert_eq!(result, Vec::<&str>::new());
    }

    #[test]
    fn keep_label_excluded_even_when_present() {
        let current = vec!["ready".to_string(), "Feature".to_string()];
        let result = stale_workflow_labels(&current, "ready");
        assert_eq!(result, Vec::<&str>::new());
    }

    #[test]
    fn non_workflow_labels_excluded() {
        let current = vec![
            "Feature".to_string(),
            "blocked".to_string(),
            "type:Bug".to_string(),
        ];
        let result = stale_workflow_labels(&current, "ready");
        assert_eq!(result, Vec::<&str>::new());
    }

    #[test]
    fn single_present_workflow_label_returned_when_not_keep() {
        let current = vec!["delivered".to_string(), "Feature".to_string()];
        let result = stale_workflow_labels(&current, "ready");
        assert_eq!(result, vec!["delivered"]);
    }

    #[test]
    fn multi_workflow_labels_filtered_to_present_minus_keep() {
        let current = vec![
            "ready".to_string(),
            "rejected".to_string(),
            "needs fixes".to_string(),
            "Feature".to_string(),
            "blocked".to_string(),
        ];
        let result = stale_workflow_labels(&current, "ready");
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"rejected"));
        assert!(result.contains(&"needs fixes"));
    }

    #[test]
    fn worst_case_all_workflow_labels_present_with_one_keep() {
        let current: Vec<String> = ALL_STATE_LABELS.iter().map(|s| (*s).to_string()).collect();
        let result = stale_workflow_labels(&current, "accepted");
        assert_eq!(result.len(), ALL_STATE_LABELS.len() - 1);
        assert!(!result.contains(&"accepted"));
    }

    #[test]
    fn empty_keep_keeps_all_present_workflow_labels() {
        let current = vec![
            "ready".to_string(),
            "delivered".to_string(),
            "Feature".to_string(),
        ];
        let result = stale_workflow_labels(&current, KEEP_NONE);
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"ready"));
        assert!(result.contains(&"delivered"));
    }
}
