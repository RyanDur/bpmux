use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::broadcast;

use super::input_activity::InputActivity;
use super::output_reader::spawn_output_reader;
use super::paste_mode::frame_for_paste;
use super::process::{io_other, signal};

const PTY_OUTPUT_CAPACITY: usize = 1024;
const SUBMIT: &[u8] = b"\r";

pub fn spawn_pty(command: &str, args: &[String]) -> std::io::Result<u32> {
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(io_other)?;
    let mut cmd = CommandBuilder::new(command);
    for a in args {
        cmd.arg(a);
    }
    let child = pair.slave.spawn_command(cmd).map_err(io_other)?;
    drop(pair.slave);
    drop(pair.master);
    let pid = child.process_id().unwrap_or(0);
    std::mem::forget(child);
    Ok(pid)
}

pub struct LivePty {
    pid: u32,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Mutex<Box<dyn std::io::Write + Send>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    output: broadcast::Sender<Vec<u8>>,
    paste_capable: Arc<AtomicBool>,
    current_paste_mode: Arc<AtomicBool>,
    human_input: InputActivity,
    _retained_tempdir: Option<tempfile::TempDir>,
}

impl LivePty {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn attach_tempdir(&mut self, dir: tempfile::TempDir) {
        self._retained_tempdir = Some(dir);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.output.subscribe()
    }

    pub fn write_input(&self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn note_human_input(&self) {
        self.human_input.note();
    }

    pub fn human_input_age(&self) -> Option<Duration> {
        self.human_input.age()
    }

    pub fn paste_capable(&self) -> bool {
        self.paste_capable.load(Ordering::Relaxed)
    }

    pub fn current_bracketed_paste(&self) -> bool {
        self.current_paste_mode.load(Ordering::Relaxed)
    }

    pub fn submit_input(&self, line: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let framed = frame_for_paste(line, self.paste_capable());
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        writer.write_all(&framed)?;
        writer.write_all(SUBMIT)?;
        writer.flush()
    }

    pub fn resize(&self, rows: u16, cols: u16) -> std::io::Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(io_other)
    }

    pub fn has_foreground_job(&self) -> bool {
        matches!(self.master.process_group_leader(), Some(pgid) if pgid != self.pid as i32)
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    pub fn close_and_reap(mut self) {
        self.graceful_stop();
    }

    fn graceful_stop(&mut self) {
        let _ = signal(self.pid, "TERM");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while std::time::Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for LivePty {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn resolve_terminal_spawn(
    room_agent: Option<&str>,
    shell: Option<&str>,
) -> (String, Vec<String>) {
    if let Some(agent) = room_agent.filter(|s| !s.is_empty()) {
        return (agent.to_string(), Vec::new());
    }
    if let Some(sh) = shell.filter(|s| !s.is_empty()) {
        return (sh.to_string(), vec!["-l".to_string()]);
    }
    ("/bin/sh".to_string(), Vec::new())
}

pub fn spawn_pty_retained(
    command: &str,
    args: &[String],
    cwd: Option<&std::path::Path>,
    env: &[(&str, &str)],
) -> std::io::Result<LivePty> {
    let system = native_pty_system();
    let pair = system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(io_other)?;
    let mut cmd = CommandBuilder::new(command);
    for a in args {
        cmd.arg(a);
    }
    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }
    for (key, value) in env {
        cmd.env(key, value);
    }
    let child = pair.slave.spawn_command(cmd).map_err(io_other)?;
    drop(pair.slave);
    let pid = child.process_id().unwrap_or(0);
    let writer = pair.master.take_writer().map_err(io_other)?;
    let reader = pair.master.try_clone_reader().map_err(io_other)?;
    let (output, _) = broadcast::channel(PTY_OUTPUT_CAPACITY);
    let paste_capable = Arc::new(AtomicBool::new(false));
    let current_paste_mode = Arc::new(AtomicBool::new(false));
    spawn_output_reader(
        reader,
        output.clone(),
        paste_capable.clone(),
        current_paste_mode.clone(),
    );
    Ok(LivePty {
        pid,
        master: pair.master,
        writer: Mutex::new(writer),
        child,
        output,
        paste_capable,
        current_paste_mode,
        human_input: InputActivity::default(),
        _retained_tempdir: None,
    })
}
