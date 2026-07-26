use std::collections::HashSet;
use std::sync::Arc;

use crate::{SurfacedTopicsRepo, db};
use database_support::DatabaseGateway;

pub struct TopicsDataGateway {
    gateway: Arc<DatabaseGateway>,
}

impl TopicsDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>) -> Self {
        Self { gateway }
    }
}

impl SurfacedTopicsRepo for TopicsDataGateway {
    fn load_surfaced_topics(&self) -> HashSet<String> {
        db::load(&self.gateway.lock())
    }

    fn save_surfaced_topics(&self, topics: &HashSet<String>) -> Result<(), std::io::Error> {
        db::replace(&self.gateway.lock(), topics)
    }
}
