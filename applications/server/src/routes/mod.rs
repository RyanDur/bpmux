use std::sync::Arc;

mod gated;
mod test;

pub(super) use gated::gated_routes;
pub(super) use test::{test_routes, test_seed_routes};

type HubRouter = axum::Router<Arc<hub::Hub>>;
