//! Legacy priority.json path resolver.
//!
//! Pre-Cassiopeia, this module owned per-state-column positional
//! ordering. All three substrates retired across #1301 slices 3, 4A,
//! and 4E. The only surface that remains is the path resolver — used
//! by the boot migration [`crate::migrate_legacy_ready_to_labels`] to
//! locate the legacy `~/.bpmux/priority.json` file for one-shot
//! readout of pre-Cassiopeia `columns["ready"]` entries.
//!
//! Post-migration, priority.json is effectively dead state and can be
//! safely removed by the user; the migration is idempotent so a
//! missing file on subsequent boots is a no-op.

use std::path::PathBuf;

/// Default priority file path. Honors `BPMUX_PRIORITY_PATH` for per-
/// test isolation; falls back to `~/.bpmux/priority.json` in prod.
pub fn default_priority_path() -> Result<PathBuf, String> {
    if let Ok(override_path) = std::env::var("BPMUX_PRIORITY_PATH")
        && !override_path.is_empty()
    {
        return Ok(PathBuf::from(override_path));
    }
    let home = dirs::home_dir().ok_or_else(|| "cannot determine home directory".to_string())?;
    Ok(home.join(".bpmux").join("priority.json"))
}
