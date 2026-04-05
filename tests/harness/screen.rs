//! Rebuildable terminal screen: accumulates raw PTY bytes and renders
//! them on demand. Kept separate from PTY-spawning code so the rendering
//! behavior can be tested without spawning a process.

use ratatui_testlib::ScreenState;

const COLS: u16 = 80;
const ROWS: u16 = 24;

pub struct Screen {
    bytes: Vec<u8>,
}

impl Screen {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.bytes.extend_from_slice(chunk);
    }

    /// Render the current screen by replaying every byte into a fresh
    /// `ScreenState`. Rebuilding each time avoids mid-redraw races where a
    /// persistent state machine catches a clear-screen escape before the
    /// following frame lands.
    pub fn contents(&self) -> String {
        let mut state = ScreenState::new(COLS, ROWS);
        state.feed(&self.bytes);
        state.contents()
    }
}

impl Default for Screen {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contents_reflects_fed_bytes() {
        let mut screen = Screen::new();
        screen.feed(b"hello world");
        assert!(screen.contents().contains("hello world"));
    }

    #[test]
    fn fresh_screen_is_empty() {
        let screen = Screen::new();
        let rendered = screen.contents();
        assert!(!rendered.contains("hello"));
    }

    #[test]
    fn rebuild_survives_redraw_cycle() {
        // A TUI redraws by clearing and rewriting. Mid-frame, the state
        // machine can look empty. Since `contents` rebuilds from all bytes,
        // queries see the final frame, not an intermediate blank state.
        let mut screen = Screen::new();
        screen.feed(b"first frame");
        assert!(screen.contents().contains("first frame"));
        screen.feed(b"\x1b[2J\x1b[H");
        screen.feed(b"second frame");
        assert!(screen.contents().contains("second frame"));
        assert!(!screen.contents().contains("first frame"));
    }

    #[test]
    fn contents_is_idempotent() {
        let mut screen = Screen::new();
        screen.feed(b"stable");
        let first = screen.contents();
        let second = screen.contents();
        assert_eq!(first, second);
    }
}
