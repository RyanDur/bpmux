use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use portable_pty::PtySize;
use tokio::sync::broadcast;

use super::input_activity::InputActivity;
use super::paste_mode::frame_for_paste;
use super::process::{io_other, signal};

const SUBMIT: &[u8] = b"\r";

pub struct LivePty {
    pub(super) pid: u32,
    pub(super) master: Box<dyn portable_pty::MasterPty + Send>,
    pub(super) writer: Mutex<Box<dyn std::io::Write + Send>>,
    pub(super) child: Box<dyn portable_pty::Child + Send + Sync>,
    pub(super) output: broadcast::Sender<Vec<u8>>,
    pub(super) paste_capable: Arc<AtomicBool>,
    pub(super) current_paste_mode: Arc<AtomicBool>,
    pub(super) human_input: InputActivity,
    pub(super) _retained_tempdir: Option<tempfile::TempDir>,
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
