fn current_data_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = location::resolve_default_base()?;
    Ok(location::durable_data_dir(&base)?)
}

pub fn run_migrate_storage() -> anyhow::Result<()> {
    let data_dir = current_data_dir()?;
    let report = storage::migrate_file_log_to_sqlite(&data_dir)?;
    if report.already_complete {
        println!(
            "migration already complete — no-op (the jsonl message log is already in the sqlite store)"
        );
    } else {
        println!(
            "migrated {} message(s) into the sqlite store; dropped {} ephemeral board_snapshot delta(s)",
            report.migrated, report.filtered
        );
    }
    let routing = storage::migrate_routing_identity_to_agents(&data_dir)?;
    if routing.already_complete {
        println!(
            "routing-identity migration already complete — no-op (the agents table already holds the routing identities)"
        );
    } else {
        println!(
            "migrated {} routing identit(y/ies) — display + durable drain cursor — from routing_identity.json into the sqlite agents table; cursors are preserved so agents don't re-flood history after the flip",
            routing.migrated
        );
    }
    Ok(())
}
