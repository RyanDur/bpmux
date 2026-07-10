mod path;

use path::diag_path;
#[cfg(test)]
pub(crate) use path::{path_has_deps_segment, resolve_diag_path};

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Mutex;

pub trait DiagSink {
    fn append(&self, line: &str);
    fn read(&self) -> String;
}

pub struct FileDiagSink;

impl DiagSink for FileDiagSink {
    fn append(&self, line: &str) {
        let Some(path) = diag_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        else {
            return;
        };
        let ts = chrono::Utc::now().to_rfc3339();
        let _ = writeln!(file, "{ts} {line}");
    }

    fn read(&self) -> String {
        diag_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .unwrap_or_default()
    }
}

#[derive(Default)]
pub struct MemoryDiagSink {
    buffer: Mutex<String>,
}

impl MemoryDiagSink {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DiagSink for MemoryDiagSink {
    fn append(&self, line: &str) {
        let ts = chrono::Utc::now().to_rfc3339();
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push_str(&format!("{ts} {line}\n"));
        }
    }

    fn read(&self) -> String {
        self.buffer
            .lock()
            .map(|buffer| buffer.clone())
            .unwrap_or_default()
    }
}

thread_local! {
    static CURRENT_SINK: RefCell<Option<Rc<dyn DiagSink>>> = const { RefCell::new(None) };
}

pub fn install_thread_sink(sink: Rc<dyn DiagSink>) {
    CURRENT_SINK.with(|cell| *cell.borrow_mut() = Some(sink));
}

pub fn clear_thread_sink() {
    CURRENT_SINK.with(|cell| *cell.borrow_mut() = None);
}

pub fn log_diag(line: &str) {
    CURRENT_SINK.with(|cell| match cell.borrow().as_ref() {
        Some(sink) => sink.append(line),
        None => FileDiagSink.append(line),
    });
}

#[cfg(test)]
mod diag_tests;
