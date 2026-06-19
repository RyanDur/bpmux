use super::json_shape::gh_state_label;
use octocrab::models::IssueState;

#[test]
fn gh_state_label_maps_open_and_closed() {
    assert_eq!(gh_state_label(&IssueState::Open), "open");
    assert_eq!(gh_state_label(&IssueState::Closed), "closed");
}
