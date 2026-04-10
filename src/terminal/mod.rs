pub mod app;
mod board;
mod chat;
mod commands;
pub mod event;
mod input;
mod markdown;
mod p2p;
mod sidebar;
mod table;
mod theme;
pub mod ui;
pub mod ui_state;

pub use app::App;
pub use event::run_tui;

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod ui_tests;
