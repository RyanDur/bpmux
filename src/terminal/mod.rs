pub mod app;
mod commands;
pub mod event;
mod input;
mod sidebar;
pub mod ui;

pub use app::App;
pub use event::run_tui;

#[cfg(test)]
mod app_tests;
#[cfg(test)]
mod ui_tests;
