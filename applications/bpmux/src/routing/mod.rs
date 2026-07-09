mod cursor_time;
mod identity;
mod naming;
pub mod slice;
mod user;

pub use identity::*;
pub use naming::bridge_agent_name;
pub(crate) use naming::short_name;
pub(crate) use user::default_kind;
pub use user::*;

#[cfg(test)]
mod identity_tests;
#[cfg(test)]
mod user_tests;
