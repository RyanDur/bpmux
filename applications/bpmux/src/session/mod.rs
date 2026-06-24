pub mod aggregate;
pub mod location;

#[cfg(test)]
mod team_roster_tests;

pub use aggregate::{BROADCAST_CHANNEL_CAPACITY, Session};
pub use location::{
    DirSession, create_session, default_sessions_base, read_or_create_dir_session,
    resume_or_create_session,
};

#[cfg(test)]
mod aggregate_tests;
#[cfg(test)]
mod location_tests;
