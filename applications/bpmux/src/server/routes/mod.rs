use std::sync::Arc;

use crate::session;

mod gated;
mod test;

pub(super) use gated::gated_routes;
pub(super) use test::{test_routes, test_seed_routes};

type SessionRouter = axum::Router<Arc<session::Session>>;
