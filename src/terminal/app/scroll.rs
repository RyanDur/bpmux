use super::App;

impl App {
    pub fn scroll_up(&mut self, lines: usize) {
        let before = self.scroll.offset;
        self.scroll.offset = self
            .scroll
            .offset
            .saturating_add(lines)
            .min(self.scroll.max);
        if self.scroll.offset != before {
            self.scroll.at_boundary = false;
        }
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let before = self.scroll.offset;
        self.scroll.offset = self.scroll.offset.saturating_sub(lines);
        if self.scroll.offset != before {
            self.scroll.at_boundary = false;
        }
        if self.scroll.offset == 0 {
            self.scroll.unread_count = 0;
        }
    }
}
