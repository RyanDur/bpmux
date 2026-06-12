const ENABLE: &[u8] = b"\x1b[?2004h";
const DISABLE: &[u8] = b"\x1b[?2004l";
const TOGGLE_LEN: usize = ENABLE.len();

pub const PASTE_START: &[u8] = b"\x1b[200~";
pub const PASTE_END: &[u8] = b"\x1b[201~";

#[derive(Default)]
pub struct PasteModeSniffer {
    carry: Vec<u8>,
}

impl PasteModeSniffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, chunk: &[u8]) -> Option<bool> {
        let mut scan = std::mem::take(&mut self.carry);
        scan.extend_from_slice(chunk);
        let state = last_toggle(&scan);
        let keep = scan.len().saturating_sub(TOGGLE_LEN - 1);
        self.carry = scan.split_off(keep);
        state
    }
}

fn last_toggle(bytes: &[u8]) -> Option<bool> {
    let mut state = None;
    let mut i = 0;
    while i + TOGGLE_LEN <= bytes.len() {
        let window = &bytes[i..i + TOGGLE_LEN];
        if window == ENABLE {
            state = Some(true);
            i += TOGGLE_LEN;
        } else if window == DISABLE {
            state = Some(false);
            i += TOGGLE_LEN;
        } else {
            i += 1;
        }
    }
    state
}

pub fn frame_for_paste(line: &[u8], paste_mode: bool) -> Vec<u8> {
    if !paste_mode {
        return line.to_vec();
    }
    let mut framed = Vec::with_capacity(PASTE_START.len() + line.len() + PASTE_END.len());
    framed.extend_from_slice(PASTE_START);
    framed.extend_from_slice(line);
    framed.extend_from_slice(PASTE_END);
    framed
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
}
