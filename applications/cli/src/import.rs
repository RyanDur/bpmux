use std::path::Path;

fn current_data_dir() -> anyhow::Result<std::path::PathBuf> {
    let base = location::resolve_default_base()?;
    Ok(location::durable_data_dir(&base)?)
}

pub fn run_import_conversation(source: &Path) -> anyhow::Result<()> {
    let data_dir = current_data_dir()?;
    let report = storage::import_conversation(&data_dir, source)?;
    println!(
        "imported {} message(s) into the conversation; skipped {} already present; \
         dropped {} ephemeral board delta(s); {} unparseable line(s); \
         conversation order restored by created-at",
        report.imported,
        report.skipped_existing,
        report.filtered_ephemeral,
        report.unparseable_lines
    );
    Ok(())
}
