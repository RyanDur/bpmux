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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn osc7(host: &str, path: &str) -> Vec<u8> {
        format!("\x1b]7;file://{host}{path}\x07").into_bytes()
    }

    #[test]
    fn a_complete_osc7_in_one_chunk_yields_the_cwd_path() {
        let mut parser = Osc7CwdParser::new();
        let got = parser.feed(&osc7("myhost", "/Users/ryan/project"));
        assert_eq!(
            got.as_deref(),
            Some("/Users/ryan/project"),
            "a complete OSC-7 (ESC]7;file://HOST/PATH BEL) yields the cwd PATH (host stripped); got {got:?}"
        );
    }

    #[test]
    fn an_osc7_split_across_a_read_boundary_still_yields_the_cwd() {
        let full = osc7("myhost", "/Users/ryan/split-dir");
        let split = full.len() / 2;
        let mut parser = Osc7CwdParser::new();
        assert_eq!(
            parser.feed(&full[..split]),
            None,
            "the first half (an incomplete sequence) yields no cwd yet — it must be buffered, not dropped"
        );
        let got = parser.feed(&full[split..]);
        assert_eq!(
            got.as_deref(),
            Some("/Users/ryan/split-dir"),
            "the second half completes the buffered sequence → the cwd parses across the 4096-read-boundary split (acceptance-4: a boundary-split cwd-emission must still register); got {got:?}"
        );
    }

    #[test]
    fn the_prefix_itself_split_across_two_chunks_still_yields_the_cwd() {
        let full = osc7("h", "/work/dir");
        let mut parser = Osc7CwdParser::new();
        assert_eq!(
            parser.feed(&full[..4]),
            None,
            "a partial PREFIX must be buffered"
        );
        let got = parser.feed(&full[4..]);
        assert_eq!(
            got.as_deref(),
            Some("/work/dir"),
            "even the OSC-7 prefix split across the boundary reassembles; got {got:?}"
        );
    }

    #[test]
    fn plain_output_with_no_osc7_yields_none_and_does_not_grow_unbounded() {
        let mut parser = Osc7CwdParser::new();
        for _ in 0..1000 {
            assert_eq!(
                parser.feed(b"plain shell output line with no escape sequence\n"),
                None,
                "non-OSC-7 output yields no cwd"
            );
        }
        assert!(
            parser.buf.len() <= MAX_BUFFERED,
            "a stream with no OSC-7 must not buffer unbounded — only a short split-prefix tail is retained; buf len {}",
            parser.buf.len()
        );
    }

    #[test]
    fn an_osc7_interleaved_with_other_output_yields_the_cwd() {
        let mut stream = b"some output before ".to_vec();
        stream.extend_from_slice(&osc7("host", "/a/b"));
        stream.extend_from_slice(b" and after");
        let mut parser = Osc7CwdParser::new();
        let got = parser.feed(&stream);
        assert_eq!(
            got.as_deref(),
            Some("/a/b"),
            "an OSC-7 embedded in surrounding output is extracted; got {got:?}"
        );
    }

    #[test]
    fn the_latest_cwd_wins_when_two_osc7_arrive_in_one_chunk() {
        let mut stream = osc7("host", "/first/dir");
        stream.extend_from_slice(&osc7("host", "/second/dir"));
        let mut parser = Osc7CwdParser::new();
        assert_eq!(
            parser.feed(&stream).as_deref(),
            Some("/second/dir"),
            "two cd's in one read → the LAST cwd is the current one"
        );
    }

    #[test]
    fn a_st_terminated_osc7_also_parses() {
        let mut parser = Osc7CwdParser::new();
        let seq = b"\x1b]7;file://host/st/dir\x1b\\";
        assert_eq!(
            parser.feed(seq).as_deref(),
            Some("/st/dir"),
            "OSC-7 terminated by ST (ESC backslash) parses as well as BEL"
        );
    }

    #[test]
    fn an_empty_host_form_yields_the_absolute_path() {
        let mut parser = Osc7CwdParser::new();
        assert_eq!(
            parser.feed(&osc7("", "/no/host")).as_deref(),
            Some("/no/host"),
            "file:///path (empty host) yields the absolute path"
        );
    }
}
