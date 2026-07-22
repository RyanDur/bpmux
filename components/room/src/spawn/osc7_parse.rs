const OSC7_PREFIX: &[u8] = b"\x1b]7;file://";
const BEL: u8 = 0x07;
const ESC: u8 = 0x1b;
const BACKSLASH: u8 = b'\\';
const MAX_BUFFERED: usize = 8192;

#[derive(Default)]
pub struct Osc7CwdParser {
    buf: Vec<u8>,
}

impl Osc7CwdParser {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Option<String> {
        self.buf.extend_from_slice(chunk);
        let mut latest = None;
        while let Some(start) = find_subslice(&self.buf, OSC7_PREFIX) {
            let payload_start = start + OSC7_PREFIX.len();
            match find_terminator(&self.buf[payload_start..]) {
                Some((offset, term_len)) => {
                    if let Some(path) =
                        extract_path(&self.buf[payload_start..payload_start + offset])
                    {
                        latest = Some(path);
                    }
                    self.buf.drain(..payload_start + offset + term_len);
                }
                None => {
                    self.buf.drain(..start);
                    break;
                }
            }
        }
        if find_subslice(&self.buf, OSC7_PREFIX).is_none() {
            keep_tail(&mut self.buf, OSC7_PREFIX.len());
        }
        if self.buf.len() > MAX_BUFFERED {
            keep_tail(&mut self.buf, OSC7_PREFIX.len());
        }
        latest
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn find_terminator(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes.iter().enumerate().find_map(|(i, &b)| {
        if b == BEL {
            Some((i, 1))
        } else if b == ESC && bytes.get(i + 1) == Some(&BACKSLASH) {
            Some((i, 2))
        } else {
            None
        }
    })
}

fn extract_path(payload: &[u8]) -> Option<String> {
    let host_and_path = std::str::from_utf8(payload).ok()?;
    let slash = host_and_path.find('/')?;
    Some(host_and_path[slash..].to_string())
}

fn keep_tail(buf: &mut Vec<u8>, keep: usize) {
    if buf.len() > keep {
        buf.drain(..buf.len() - keep);
    }
}
