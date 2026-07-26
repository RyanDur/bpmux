use std::path::Path;
use std::sync::Arc;

use awareness::SurfacedTopicsRepo;
use awareness::data_gateway::TopicsDataGateway;
use database_support::DatabaseGateway;
use memory::lessons::{LessonDataGateway, LessonRepo};
use message_log::MessageLog;
use message_log::data_gateway::ConversationDataGateway;
use product_management::data_gateway::TransitionsDataGateway;
use source_sync::OutboxRepo;
use source_sync::data_gateway::OutboxDataGateway;
use user_routing::UserRepo;
use user_routing::data_gateway::UserDataGateway;

mod instrumented;
mod meta;
mod migrate;

pub use migrate::{
    MigrationError, MigrationReport, RoutingMigrationReport, migrate_file_log_to_sqlite,
    migrate_routing_identity_to_agents,
};

#[cfg(test)]
mod boundary_tests;
#[cfg(test)]
mod migrate_tests;
#[cfg(test)]
mod open_tests;
#[cfg(test)]
mod store_behavior_tests;

#[derive(Clone)]
pub struct DataGateways {
    pub gateway: Arc<DatabaseGateway>,
    pub conversation: Arc<dyn MessageLog>,
    pub users: Arc<dyn UserRepo>,
    pub outbox: Arc<dyn OutboxRepo>,
    pub topics: Arc<dyn SurfacedTopicsRepo>,
    pub lessons: Arc<dyn LessonRepo>,
    pub transitions: Arc<TransitionsDataGateway>,
}

pub fn open(dir: &Path) -> DataGateways {
    let gateway = Arc::new(DatabaseGateway::open(dir).unwrap_or_else(|e| {
        diag::log_diag(&format!("sqlite store failed to open at {dir:?}: {e}"));
        eprintln!("bpmux: the store is the engine — cannot run without its database ({e})");
        std::process::exit(1);
    }));
    if let Err(e) = user_routing::db::seed_system(&gateway.lock()) {
        diag::log_diag(&format!("system-user seed failed: {e}"));
    }
    inherit_file_history(dir, &gateway);
    DataGateways {
        conversation: Arc::new(instrumented::InstrumentedConversation::new(
            ConversationDataGateway::new(gateway.clone(), dir.to_path_buf()),
        )),
        users: Arc::new(UserDataGateway::new(gateway.clone())),
        outbox: Arc::new(OutboxDataGateway::new(gateway.clone())),
        topics: Arc::new(TopicsDataGateway::new(gateway.clone())),
        lessons: Arc::new(LessonDataGateway::new(dir.to_path_buf())),
        transitions: Arc::new(TransitionsDataGateway::new(gateway.clone())),
        gateway,
    }
}

fn inherit_file_history(dir: &Path, gateway: &Arc<DatabaseGateway>) {
    match migrate::migrate_file_log_with(dir, gateway) {
        Ok(report) if !report.already_complete => diag::log_diag(&format!(
            "sqlite backend inherited {} message(s) from the jsonl log ({} ephemeral delta(s) dropped)",
            report.migrated, report.filtered
        )),
        Ok(_) => {}
        Err(e) => diag::log_diag(&format!(
            "jsonl history inheritance failed ({e}); the file log is untouched — run `bpmux migrate` to retry"
        )),
    }
    match migrate::migrate_routing_identity_with(dir, gateway) {
        Ok(report) if !report.already_complete => diag::log_diag(&format!(
            "sqlite backend inherited {} routing identit(y/ies) from routing_identity.json",
            report.migrated
        )),
        Ok(_) => {}
        Err(e) => diag::log_diag(&format!(
            "routing-identity inheritance failed ({e}); run `bpmux migrate` to retry"
        )),
    }
}
