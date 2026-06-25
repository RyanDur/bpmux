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
