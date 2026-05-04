use std::sync::Arc;
use std::sync::Mutex;

use chrono::Utc;

use crate::bridge::tools::CiStateReport;

#[derive(Clone, Default)]
pub struct CiStateStore {
    inner: Arc<Mutex<Option<CiStateReport>>>,
}

impl CiStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&self, mut report: CiStateReport) {
        report.last_updated_at = Some(Utc::now().to_rfc3339());
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Some(report);
    }

    pub fn read(&self) -> Option<CiStateReport> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.clone()
    }
}

pub type CiStateStoreHandle = CiStateStore;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::bridge::tools::CiStateReport;

    fn sample(conclusion: &'static str) -> CiStateReport {
        CiStateReport {
            conclusion,
            last_run_id: Some(42),
            branch: "main",
            url: "https://github.com/RyanDur/bpmux/actions".to_string(),
            head_sha: Some("abc1234".to_string()),
            display_title: Some("test".to_string()),
            last_updated_at: None,
            poll_interval_secs: 60,
            backoff_cap_secs: 3600,
        }
    }

    #[test]
    fn empty_store_returns_none() {
        let store = CiStateStore::new();
        assert!(store.read().is_none());
    }

    #[test]
    fn write_then_read_returns_report_with_last_updated_at_set() {
        let store = CiStateStore::new();
        store.write(sample("success"));
        let report = store.read().expect("after write read returns Some");
        assert_eq!(report.conclusion, "success");
        assert!(
            report.last_updated_at.is_some(),
            "write must stamp last_updated_at"
        );
        let ts = report.last_updated_at.as_deref().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
            "last_updated_at must be RFC3339: {ts}"
        );
    }

    #[test]
    fn write_overwrites_prior_state() {
        let store = CiStateStore::new();
        store.write(sample("failure"));
        store.write(sample("success"));
        let report = store.read().expect("Some after write");
        assert_eq!(report.conclusion, "success");
    }

    #[test]
    fn clone_shares_inner_state() {
        let store_a = CiStateStore::new();
        let store_b = store_a.clone();
        store_a.write(sample("success"));
        let report = store_b
            .read()
            .expect("clone must observe writes through shared Arc");
        assert_eq!(report.conclusion, "success");
    }

    #[test]
    fn write_overwrites_provided_last_updated_at() {
        let mut report = sample("success");
        report.last_updated_at = Some("2020-01-01T00:00:00+00:00".to_string());
        let store = CiStateStore::new();
        store.write(report);
        let stored = store.read().unwrap();
        let ts = stored.last_updated_at.as_deref().unwrap();
        let parsed = chrono::DateTime::parse_from_rfc3339(ts).unwrap();
        let now = chrono::Utc::now();
        let elapsed_seconds = now
            .signed_duration_since(parsed.with_timezone(&chrono::Utc))
            .num_seconds()
            .abs();
        assert!(
            elapsed_seconds < 5,
            "store stamps with current time on write, ignoring caller-provided ts: parsed={parsed} now={now}"
        );
    }
}
