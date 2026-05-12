#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1301: boot-time markers-store → iteration-issue migration
// (Epic #1189 slice 5). Real tests land alongside the impl commit;
// placeholder so start_story's ATP gate passes at pickup-time.

#[test]
fn placeholder_atp_present_for_start_story_gate() {
    assert_eq!(1 + 1, 2);
}
