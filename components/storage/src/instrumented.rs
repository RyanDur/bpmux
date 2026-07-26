use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use message::Message;
use message_log::{LogError, MessageLog};

pub struct InstrumentedConversation<L: MessageLog> {
    inner: L,
    log_path: Option<PathBuf>,
}

impl<L: MessageLog> InstrumentedConversation<L> {
    pub fn new(inner: L) -> Self {
        let log_path = std::env::var("BPMUX_STORAGE_LOG").ok().map(PathBuf::from);
        let this = Self { inner, log_path };
        if let Ok(messages) = this.inner.read_messages() {
            this.log(&format!("startup: {} messages", messages.len()));
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
            let _ = writeln!(file, "{} {line}", chrono::Utc::now().to_rfc3339());
        }
    }

    fn timed<T>(&self, label: &str, run: impl FnOnce(&L) -> T) -> T {
        let start = Instant::now();
        let result = run(&self.inner);
        self.log(&format!(
            "{label} {:.1}ms",
            start.elapsed().as_secs_f64() * 1000.0
        ));
        result
    }
}

impl<L: MessageLog> MessageLog for InstrumentedConversation<L> {
    fn append_message(&self, msg: &Message) -> Result<(), LogError> {
        self.timed("append_message", |inner| inner.append_message(msg))
    }

    fn read_messages(&self) -> Result<Vec<Message>, LogError> {
        self.timed("read_messages", |inner| inner.read_messages())
    }

    fn read_recent(
        &self,
        limit: usize,
        offset: usize,
        matches: &dyn Fn(&Message) -> bool,
    ) -> Result<Vec<Message>, LogError> {
        self.timed("read_recent", |inner| {
            inner.read_recent(limit, offset, matches)
        })
    }

    fn search_conversations(
        &self,
        needle_lower: &str,
        addressee: Option<&str>,
        radius: u64,
        accept: &mut dyn FnMut(&[Message], usize) -> bool,
    ) -> Result<(), LogError> {
        self.inner
            .search_conversations(needle_lower, addressee, radius, accept)
    }

    fn read_messages_since(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Message>, LogError> {
        self.timed("read_messages_since", |inner| {
            inner.read_messages_since(cutoff)
        })
    }

    fn clear_messages(&self) -> Result<(), std::io::Error> {
        self.timed("clear_messages", |inner| inner.clear_messages())
    }
}
