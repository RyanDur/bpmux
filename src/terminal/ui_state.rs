//! UI state persistence — Memento pattern.
//!
//! Captures and restores App view state across restarts.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::app::App;
use crate::session::Session;

#[derive(Serialize, Deserialize, Default)]
pub struct UiState {
    pub current_view: Option<String>,
    pub send_target: Option<String>,
    pub sidebar_open: bool,
    pub sidebar_focused: bool,
    pub project_expanded: bool,
    pub direct_expanded: bool,
    pub groups_expanded: bool,
    pub system_expanded: bool,
    pub scroll_offset: usize,
    pub expanded_messages: HashSet<String>,
    pub show_timestamps: bool,
    pub input: String,
    /// #161: persist the classifier-driven view filter state across
    /// hot-reloads and session restarts. Default `false` (filter on).
    /// Serde default so pre-#161 saved state loads cleanly without
    /// a migration step.
    #[serde(default)]
    pub show_all_classes: bool,
}

impl UiState {
    pub fn capture(app: &App) -> Self {
        Self {
            current_view: app.current_view.clone(),
            send_target: app.send_target.clone(),
            sidebar_open: app.sidebar.open,
            sidebar_focused: false,
            project_expanded: app.sidebar.project_expanded,
            direct_expanded: app.sidebar.direct_expanded,
            groups_expanded: app.sidebar.groups_expanded,
            system_expanded: app.sidebar.system_expanded,
            scroll_offset: app.scroll.offset,
            expanded_messages: app.expanded_messages.clone(),
            show_timestamps: app.display.show_timestamps,
            input: app.input.clone(),
            show_all_classes: app.display.show_all_classes,
        }
    }

    pub fn restore(&self, app: &mut App) {
        app.current_view = self.current_view.clone();
        app.send_target = self.send_target.clone();
        app.sidebar.open = self.sidebar_open;
        app.sidebar.project_expanded = self.project_expanded;
        app.sidebar.direct_expanded = self.direct_expanded;
        app.sidebar.groups_expanded = self.groups_expanded;
        app.sidebar.system_expanded = self.system_expanded;
        app.scroll.offset = self.scroll_offset;
        app.expanded_messages = self.expanded_messages.clone();
        app.display.show_timestamps = self.show_timestamps;
        app.display.show_all_classes = self.show_all_classes;
        app.mark_filters_dirty();
        if !self.input.is_empty() {
            app.set_textarea_content(&self.input);
        }
    }

    pub fn save(app: &App, session: &Session) {
        let state = Self::capture(app);
        if let Ok(json) = serde_json::to_string(&state) {
            session.storage.save_state("ui-state", &json);
        }
    }

    pub fn load_and_restore(app: &mut App, session: &Session) {
        if let Some(json) = session.storage.load_state("ui-state")
            && let Ok(state) = serde_json::from_str::<UiState>(&json)
        {
            state.restore(app);
        }
    }
}
