use crate::session::Message;
use crate::terminal::app::App;

impl App {
    /// `/all` — toggle the classifier-driven view filter (#161).
    /// Default state: filter ON, Routine + bpmux-nudges hidden from your
    /// pane. `/all` once flips to "show everything" mode. `/all` again
    /// flips back to filtered.
    ///
    /// No system_message — the status bar already shows "N hidden" /
    /// "showing all" so the chat-pane line was redundant feedback (the
    /// "noise about a noise filter" anti-pattern designer flagged on
    /// 2026-04-10). Removing the source beats adding a classifier rule.
    pub(in crate::terminal) fn handle_toggle_all(&mut self) -> Option<Message> {
        self.display.show_all_classes = !self.display.show_all_classes;
        self.mark_filters_dirty();
        self.clear_input();
        None
    }
}
