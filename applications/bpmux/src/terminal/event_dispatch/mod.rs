//! Keyboard and mouse event dispatch extracted from event.rs.
//!
//! The entry point is [`handle_terminal_event`]: it routes a crossterm
//! `Event` to either the mouse or key handler (or handles focus /
//! paste inline). Mouse and key handling live in sibling submodules
//! so each file focuses on one input concern.

mod key;
mod mouse;

use crossterm::event::{Event, KeyEventKind};

use crate::session;
use crate::terminal::app::App;

pub(super) fn handle_terminal_event(
    ev: Event,
    app: &mut App,
    session: &session::Session,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if matches!(&ev, Event::FocusGained) {
        app.display.tui_focused = true;
        return Ok(());
    }
    if matches!(&ev, Event::FocusLost) {
        app.display.tui_focused = false;
        return Ok(());
    }
    if let Event::Mouse(m) = &ev {
        return mouse::handle(m, app);
    }
    if let Event::Paste(text) = ev {
        for ch in text.chars() {
            app.insert_char(ch);
        }
        return Ok(());
    }
    let Event::Key(key) = ev else {
        return Ok(());
    };
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }
    key::handle(key, app, session)
}
