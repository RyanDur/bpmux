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
