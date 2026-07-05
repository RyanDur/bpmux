use std::io::Write;
use std::path::PathBuf;

use super::Storage;

pub struct InstrumentedStorage<S: Storage> {
    inner: S,
    log_path: Option<PathBuf>,
}

impl<S: Storage> InstrumentedStorage<S> {
    pub fn new(inner: S) -> Self {
        let log_path = std::env::var("BPMUX_STORAGE_LOG").ok().map(PathBuf::from);
        let this = Self { inner, log_path };
        if let Ok(messages) = this.inner.read_messages() {
            let lessons = this.inner.load_lessons();
            this.log(&format!(
                "startup: {} messages, {} experiences, {} lessons",
                messages.len(),
                lessons.experiences.len(),
                lessons.lessons.len(),
            ));
        }
        this
    }

    fn log(&self, line: &str) {
        let Some(path) = &self.log_path else {
            return;
        };
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(file, "{line}");
        }
    }
}

mod lessons;
mod message_log;
mod outbox;
mod state;
mod topics;
mod users;
