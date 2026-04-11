pub mod app;
mod board;
mod chat;
mod ci;
mod commands;
mod commands_help;
mod commands_search;
mod commands_share;
mod commands_tokens;
pub mod event;
mod event_dispatch;
mod input;
mod markdown;
mod p2p;
mod sidebar;
mod status_bar;
mod table;
mod theme;
pub mod ui;
pub mod ui_state;

pub use app::App;
pub use event::run_tui;

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod commands_tests;
#[cfg(test)]
mod ui_tests;
