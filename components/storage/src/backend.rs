use std::path::Path;
use std::sync::Arc;

use super::instrumented::InstrumentedStorage;
use super::{SqliteStorage, Storage};

pub fn open(dir: &Path) -> Arc<dyn Storage> {
    let store = SqliteStorage::new(dir.to_path_buf()).unwrap_or_else(|e| {
        diag::log_diag(&format!("sqlite store failed to open at {dir:?}: {e}"));
        eprintln!("bpmux: the store is the engine — cannot run without its database ({e})");
        std::process::exit(1);
    });
    inherit_file_history(dir);
    Arc::new(InstrumentedStorage::new(store))
}

fn inherit_file_history(dir: &Path) {
    match super::migrate_file_log_to_sqlite(dir) {
        Ok(report) if !report.already_complete => diag::log_diag(&format!(
            "sqlite backend inherited {} message(s) from the jsonl log ({} ephemeral delta(s) dropped)",
            report.migrated, report.filtered
        )),
        Ok(_) => {}
        Err(e) => diag::log_diag(&format!(
            "jsonl history inheritance failed ({e}); the file log is untouched — run `bpmux migrate` to retry"
        )),
    }
    match super::migrate_routing_identity_to_agents(dir) {
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
