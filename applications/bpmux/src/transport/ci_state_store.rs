use std::sync::Arc;
use std::sync::Mutex;

use chrono::{DateTime, Utc};

use crate::bridge::tools::CiStateReport;

#[derive(Default)]
struct CiStateInner {
    report: Option<CiStateReport>,
    last_successful_poll_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Default)]
pub struct CiStateStore {
    inner: Arc<Mutex<CiStateInner>>,
}

impl CiStateStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&self, mut report: CiStateReport) {
        let now = Utc::now();
        report.last_updated_at = Some(now.to_rfc3339());
        report.last_successful_poll_at = Some(now.to_rfc3339());
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.report = Some(report);
        guard.last_successful_poll_at = Some(now);
    }

    pub fn mark_poll_succeeded(&self) {
        let now = Utc::now();
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.last_successful_poll_at = Some(now);
        if let Some(report) = guard.report.as_mut() {
            report.last_successful_poll_at = Some(now.to_rfc3339());
        }
    }

    pub fn read(&self) -> Option<CiStateReport> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.report.clone()
    }

    pub fn last_successful_poll_at(&self) -> Option<DateTime<Utc>> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.last_successful_poll_at
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
            last_successful_poll_at: None,
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
    fn write_sets_last_successful_poll_at() {
        let store = CiStateStore::new();
        store.write(sample("success"));
        let stored = store.read().expect("Some");
        assert!(
            stored.last_successful_poll_at.is_some(),
            "write must stamp last_successful_poll_at"
        );
        let ts = stored.last_successful_poll_at.as_deref().unwrap();
        assert!(
            chrono::DateTime::parse_from_rfc3339(ts).is_ok(),
            "last_successful_poll_at must be RFC3339: {ts}"
        );
        assert!(
            store.last_successful_poll_at().is_some(),
            "store also exposes timestamp via direct accessor"
        );
    }

    #[test]
    fn mark_poll_succeeded_updates_timestamp_without_writing_report() {
        let store = CiStateStore::new();
        assert!(store.last_successful_poll_at().is_none());
        store.mark_poll_succeeded();
        assert!(
            store.last_successful_poll_at().is_some(),
            "mark_poll_succeeded must stamp timestamp"
        );
        assert!(
            store.read().is_none(),
            "mark_poll_succeeded MUST NOT write a report (304-path: state unchanged)"
        );
    }

    #[test]
    fn mark_poll_succeeded_after_write_updates_timestamp_in_existing_report() {
        let store = CiStateStore::new();
        store.write(sample("success"));
        let first_ts = store
            .read()
            .unwrap()
            .last_successful_poll_at
            .expect("set on write");
        std::thread::sleep(std::time::Duration::from_millis(10));
        store.mark_poll_succeeded();
        let second_ts = store
            .read()
            .unwrap()
            .last_successful_poll_at
            .expect("set on mark_poll_succeeded");
        assert_ne!(
            first_ts, second_ts,
            "mark_poll_succeeded must advance the existing report's timestamp"
        );
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
