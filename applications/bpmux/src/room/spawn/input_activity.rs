use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct InputActivity {
    last: Mutex<Option<Instant>>,
}

impl InputActivity {
    pub(super) fn note(&self) {
        *self.last.lock().unwrap_or_else(|e| e.into_inner()) = Some(Instant::now());
    }

    pub(super) fn age(&self) -> Option<Duration> {
        self.last
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(Instant::elapsed)
    }
}
