fn current_session_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = crate::location::default_sessions_base()?;
    Ok(crate::location::resume_or_create_session(&base)?)
}

pub(crate) fn run_migrate_storage() -> anyhow::Result<()> {
    let session_dir = current_session_dir()?;
    let report = crate::storage::migrate_file_log_to_sqlite(&session_dir)?;
    if report.already_complete {
        println!(
            "migration already complete — no-op (the jsonl message log is already in the sqlite store)"
        );
    } else {
        println!(
            "migrated {} message(s) into the sqlite store; dropped {} ephemeral board_snapshot delta(s); the jsonl stays source-of-truth until you verify and flip BPMUX_STORAGE_BACKEND=sqlite",
            report.migrated, report.filtered
        );
    }
    let routing = crate::storage::migrate_routing_identity_to_agents(&session_dir)?;
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
