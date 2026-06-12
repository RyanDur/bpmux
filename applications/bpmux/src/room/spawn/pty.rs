use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::broadcast;

use super::paste_mode::{PasteModeSniffer, frame_for_paste};
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
    paste_mode: Arc<AtomicBool>,
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

    pub fn paste_mode_enabled(&self) -> bool {
        self.paste_mode.load(Ordering::Relaxed)
    }

    pub fn submit_input(&self, line: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let framed = frame_for_paste(line, self.paste_mode_enabled());
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
    let paste_mode = Arc::new(AtomicBool::new(false));
    spawn_output_reader(reader, output.clone(), paste_mode.clone());
    Ok(LivePty {
        pid,
        master: pair.master,
        writer: Mutex::new(writer),
        child,
        output,
        paste_mode,
        _retained_tempdir: None,
    })
}

fn spawn_output_reader(
    mut reader: Box<dyn std::io::Read + Send>,
    output: broadcast::Sender<Vec<u8>>,
    paste_mode: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        use std::io::Read;
        let mut sniffer = PasteModeSniffer::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = output.send(Vec::new());
                    break;
                }
                Ok(n) => {
                    if let Some(state) = sniffer.observe(&buf[..n]) {
                        paste_mode.store(state, Ordering::Relaxed);
                    }
                    let _ = output.send(buf[..n].to_vec());
                }
                Err(_) => {
                    let _ = output.send(Vec::new());
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::room::spawn::process_alive;

    #[test]
    fn spawn_pty_returns_positive_pid_for_running_child() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 1".into()]).unwrap();
        assert!(pid > 0, "spawn must return a child PID; got {pid}");
        assert!(
            process_alive(pid),
            "child {pid} must be alive immediately after spawn"
        );
    }

    #[test]
    fn spawn_pty_survives_parent_handle_drop() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 2".into()]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            process_alive(pid),
            "child {pid} must outlive the spawn handles being dropped"
        );
    }

    #[test]
    fn spawn_pty_retained_streams_child_output_to_a_subscriber() {
        let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
        assert!(live.pid() > 0, "retained-pty spawn must report a child PID");
        let mut rx = live.subscribe();
        live.write_input(b"printf 'hello-retained-pty\\n'\n")
            .unwrap();
        let collected = drain_subscriber_until(&mut rx, "hello-retained-pty");
        assert!(
            collected.contains("hello-retained-pty"),
            "the RETAINED master's server-owned reader must stream the child's live PTY output to a subscriber (not dropped like spawn_pty); got {collected:?}"
        );
    }

    fn drain_subscriber_until(rx: &mut broadcast::Receiver<Vec<u8>>, marker: &str) -> String {
        let mut out = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match rx.blocking_recv() {
                Ok(bytes) if bytes.is_empty() => break,
                Ok(bytes) => {
                    out.push_str(&String::from_utf8_lossy(&bytes));
                    if out.contains(marker) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        out
    }

    #[test]
    fn two_subscribers_to_one_pty_both_receive_the_full_output() {
        let live = spawn_pty_retained("/bin/sh", &[], None, &[]).unwrap();
        let mut first = live.subscribe();
        let mut second = live.subscribe();
        live.write_input(b"printf 'fanout-marker\\n'\n").unwrap();

        let first_out = drain_subscriber_until(&mut first, "fanout-marker");
        let second_out = drain_subscriber_until(&mut second, "fanout-marker");

        assert!(
            first_out.contains("fanout-marker"),
            "subscriber one must receive the live PTY output via the server-owned reader fanout; got {first_out:?}"
        );
        assert!(
            second_out.contains("fanout-marker"),
            "subscriber two must ALSO receive the SAME full output — one server-owned reader fans out to every subscriber, never splitting the master fd between competing per-subscriber readers; got {second_out:?}"
        );
    }

    #[test]
    fn the_output_sniffer_flips_paste_mode_when_the_agent_declares_bracketed_paste() {
        let live = spawn_pty_retained(
            "/bin/sh",
            &[
                "-c".into(),
                "read trigger; printf '\\033[?2004hPASTE-ON-MARKER\\n'; sleep 2".into(),
            ],
            None,
            &[],
        )
        .unwrap();
        assert!(
            !live.paste_mode_enabled(),
            "a freshly-spawned terminal defaults to paste-mode OFF until the agent declares it"
        );
        let mut rx = live.subscribe();
        live.write_input(b"go\n").unwrap();
        let collected = drain_subscriber_until(&mut rx, "PASTE-ON-MARKER");
        assert!(
            collected.contains("PASTE-ON-MARKER"),
            "precondition: the child's output (incl the ESC[?2004h that precedes the marker in the same write) must have streamed; got {collected:?}"
        );
        assert!(
            live.paste_mode_enabled(),
            "after the agent emits ESC[?2004h, the LivePty output-sniffer must have flipped paste-mode ON — so injected lines get bracketed-paste-framed and auto-submit (a real Enter) instead of parking in the agent's input until a manual keypress (the owner-walk reject)"
        );
    }

    #[test]
    fn spawn_pty_retained_runs_a_real_shell_in_the_given_cwd_with_term() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let cwd =
            std::env::temp_dir().join(format!("bpmux-shell-cwd-{}-{}", std::process::id(), "s1"));
        std::fs::create_dir_all(&cwd).unwrap();
        let marker = cwd.file_name().unwrap().to_string_lossy().to_string();
        let live = spawn_pty_retained(
            &shell,
            &[],
            Some(cwd.as_path()),
            &[("TERM", "xterm-256color")],
        )
        .unwrap();
        let mut rx = live.subscribe();
        live.write_input(b"echo TERM=$TERM PWD=$PWD\n").unwrap();
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match rx.blocking_recv() {
                Ok(bytes) if bytes.is_empty() => break,
                Ok(bytes) => {
                    collected.push_str(&String::from_utf8_lossy(&bytes));
                    if collected.contains("TERM=xterm-256color") && collected.contains(&marker) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = std::fs::remove_dir_all(&cwd);
        assert!(
            collected.contains("TERM=xterm-256color"),
            "the spawned shell must run with TERM=xterm-256color (interpolated by a REAL shell, not the literal echo); got {collected:?}"
        );
        assert!(
            collected.contains(&marker),
            "the spawned shell must start in the given cwd (PWD must contain the spawn dir, not the backend's cwd); got {collected:?}"
        );
    }

    #[test]
    fn live_pty_resize_propagates_the_winsize_to_the_shell_tty() {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let live = spawn_pty_retained(&shell, &[], None, &[("TERM", "xterm-256color")]).unwrap();
        live.resize(40, 120).unwrap();
        let mut rx = live.subscribe();
        live.write_input(b"stty size\n").unwrap();
        let collected = drain_subscriber_until(&mut rx, "40 120");
        assert!(
            collected.contains("40 120"),
            "stty size must report the RESIZED winsize (rows cols), proving master.resize propagated to the slave tty — not the spawn default 24 80; got {collected:?}"
        );
    }

    #[test]
    fn resolve_terminal_spawn_defaults_to_a_login_shell_when_no_override() {
        let (command, args) = resolve_terminal_spawn(None, Some("/bin/zsh"));
        assert_eq!(
            command, "/bin/zsh",
            "with no override ＋ spawns the user's $SHELL — a bare login shell (the agent RUNNING in it is the registration signal, sourced by the always-on detector, not by this spawn-target)"
        );
        assert_eq!(
            args,
            vec!["-l".to_string()],
            "the $SHELL spawns as a LOGIN shell so it loads the user's environment"
        );
    }

    #[test]
    fn resolve_terminal_spawn_honours_an_override_without_login_args() {
        let (command, args) = resolve_terminal_spawn(Some("/bin/echo"), Some("/bin/zsh"));
        assert_eq!(
            command, "/bin/echo",
            "an explicit BPMUX_ROOM_AGENT override wins over $SHELL — the spawn-command seam (the e2e harness pins a deterministic spawn-target; unset in prod, so prod ＋ is the bare $SHELL)"
        );
        assert!(
            args.is_empty(),
            "an override spawns as-is (no login args injected into an arbitrary target)"
        );
    }

    #[test]
    fn resolve_terminal_spawn_treats_empty_env_as_unset() {
        let (command, args) = resolve_terminal_spawn(Some(""), Some(""));
        assert_eq!(
            command, "/bin/sh",
            "empty env vars fall back to /bin/sh, never spawn \"\""
        );
        assert!(args.is_empty());
    }

    #[test]
    fn close_and_reap_terminates_a_running_process_no_orphan() {
        let live = spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[])
            .unwrap();
        let pid = live.pid();
        assert!(
            process_alive(pid),
            "the spawned process must be alive before close; pid {pid}"
        );
        live.close_and_reap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while process_alive(pid) && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            !process_alive(pid),
            "close_and_reap must terminate the spawned process (SIGTERM→grace→SIGKILL→reap) — pid {pid} still alive is the orphan leak this slice closes"
        );
    }

    #[test]
    fn has_foreground_job_is_false_for_a_bare_spawned_process() {
        let live = spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[])
            .unwrap();
        assert!(
            !live.has_foreground_job(),
            "a bare spawned process with no forked foreground command must report no foreground job — closing it needs no confirm-before-close"
        );
        live.close_and_reap();
    }

    #[test]
    fn is_alive_reports_true_while_running_and_false_after_the_child_exits() {
        let mut running =
            spawn_pty_retained("/bin/sh", &["-c".into(), "exec sleep 30".into()], None, &[])
                .unwrap();
        assert!(
            running.is_alive(),
            "a still-running child must report is_alive=true"
        );
        running.close_and_reap();

        let mut exiting =
            spawn_pty_retained("/bin/sh", &["-c".into(), "exit 0".into()], None, &[]).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while exiting.is_alive() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            !exiting.is_alive(),
            "after the child exits, is_alive must report false — try_wait() observes the exit and reaps the zombie, where a kill -0 pid-probe would false-positive on the lingering zombie pid that refresh-resurrection (e) depends on excluding"
        );
    }
}
