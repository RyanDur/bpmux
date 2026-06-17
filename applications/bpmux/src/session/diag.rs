use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Mutex;

const TEST_DIAG_FILENAME: &str = "bpmux-test-diag.log";

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

fn diag_path() -> Option<std::path::PathBuf> {
    let bpmux_log = std::env::var("BPMUX_LOG").ok();
    let bpmux_test_mode = std::env::var("BPMUX_TEST_MODE").ok();
    let exe_is_cargo_test = std::env::current_exe()
        .ok()
        .map(path_has_deps_segment)
        .unwrap_or(false);
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let tmpdir = std::env::temp_dir();
    resolve_diag_path(
        bpmux_log.as_deref(),
        bpmux_test_mode.as_deref(),
        exe_is_cargo_test,
        home.as_deref(),
        &tmpdir,
    )
}

pub(crate) fn resolve_diag_path(
    bpmux_log: Option<&str>,
    bpmux_test_mode: Option<&str>,
    exe_is_cargo_test: bool,
    home: Option<&std::path::Path>,
    tmpdir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    if let Some(p) = bpmux_log.filter(|s| !s.is_empty()) {
        return Some(std::path::PathBuf::from(p));
    }
    let in_test_mode = bpmux_test_mode.is_some_and(|s| !s.is_empty()) || exe_is_cargo_test;
    if in_test_mode {
        return Some(tmpdir.join(TEST_DIAG_FILENAME));
    }
    home.map(|h| h.join(".bpmux").join("diag.log"))
}

pub(crate) fn path_has_deps_segment(exe: std::path::PathBuf) -> bool {
    exe.components().any(|c| c.as_os_str() == "deps")
}
