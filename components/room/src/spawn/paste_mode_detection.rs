#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::paste_mode::*;

#[test]
fn an_enable_toggle_in_one_chunk_reports_paste_on() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(
        sniffer.observe(b"prompt \x1b[?2004h> "),
        Some(true),
        "the agent emitting ESC[?2004h declares bracketed-paste ON"
    );
}

#[test]
fn a_disable_toggle_reports_paste_off() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(
        sniffer.observe(b"\x1b[?2004l"),
        Some(false),
        "ESC[?2004l declares bracketed-paste OFF"
    );
}

#[test]
fn the_last_toggle_in_a_chunk_wins() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(
        sniffer.observe(b"\x1b[?2004h text \x1b[?2004l more"),
        Some(false),
        "two toggles in one read the LAST declared mode is current"
    );
}

#[test]
fn plain_output_reports_no_toggle() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(
        sniffer.observe(b"just a normal line of output\n"),
        None,
        "output with no toggle leaves the mode unchanged (None)"
    );
}

#[test]
fn a_toggle_split_across_a_read_boundary_still_registers() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(
        sniffer.observe(b"\x1b[?20"),
        None,
        "the first half of the toggle is incomplete buffered, not lost"
    );
    assert_eq!(
        sniffer.observe(b"04h"),
        Some(true),
        "the second half completes the buffered toggle across the read-boundary split the SAME class of boundary-reassembly the OSC-7 sniffer guarantees"
    );
}

#[test]
fn the_carry_never_re_reports_a_completed_toggle() {
    let mut sniffer = PasteModeSniffer::new();
    assert_eq!(sniffer.observe(b"\x1b[?2004h done"), Some(true));
    assert_eq!(
        sniffer.observe(b"more plain output"),
        None,
        "a toggle fully consumed in a prior chunk must not re-fire from the carry tail the carry is shorter than a full toggle, so it can only COMPLETE a split, never duplicate"
    );
}

#[test]
fn paste_on_wraps_the_line_in_a_paste_then_leaves_the_submit_to_the_caller() {
    assert_eq!(
        frame_for_paste(b"hello team", true),
        b"\x1b[200~hello team\x1b[201~".to_vec(),
        "a paste-mode agent receives the line as a real bracketed paste; the discrete CR submit is written separately by the caller so it lands OUTSIDE the paste frame as a deliberate Enter"
    );
}

#[test]
fn paste_off_passes_the_line_through_unframed() {
    assert_eq!(
        frame_for_paste(b"hello team", false),
        b"hello team".to_vec(),
        "a cooked-mode agent (a bare read-loop) must NOT receive paste markers it gets the plain line, and the caller's CR completes it via the kernel line-discipline"
    );
}
