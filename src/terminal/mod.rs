pub mod app;
mod app_state;
mod board;
pub(crate) mod board_poller;
mod board_view;
mod chat;
mod ci;
mod commands;
pub mod event;
mod event_dispatch;
mod icons;
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
mod board_poller_tests;
#[cfg(test)]
mod board_tests;
#[cfg(test)]
mod ci_tests;
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod ui_tests;
