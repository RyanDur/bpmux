use chrono::{DateTime, Utc};

pub(super) fn remote_state_supersedes_local(
    local_state_written_at: Option<DateTime<Utc>>,
    incoming_remote_updated_at: Option<DateTime<Utc>>,
) -> bool {
    match local_state_written_at {
        None => true,
        Some(local_ts) => {
            matches!(incoming_remote_updated_at, Some(remote_ts) if remote_ts >= local_ts)
        }
    }
}
