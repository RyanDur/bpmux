fn current_session_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = crate::session::default_sessions_base()?;
    Ok(crate::session::resume_or_create_session(&base)?)
}

pub(crate) fn run_migrate_storage() -> anyhow::Result<()> {
    let session_dir = current_session_dir()?;
    let report = crate::session::storage::migrate_file_log_to_sqlite(&session_dir)?;
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
    Ok(())
}
