use std::sync::Mutex;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};

pub fn spawn_pty(command: &str, args: &[String], env: &[(&str, &str)]) -> std::io::Result<u32> {
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
    for (key, value) in env {
        cmd.env(key, value);
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
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl LivePty {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn reader(&self) -> std::io::Result<Box<dyn std::io::Read + Send>> {
        self.master.try_clone_reader().map_err(io_other)
    }

    pub fn write_input(&self, bytes: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        let mut writer = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        writer.write_all(bytes)?;
        writer.flush()
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
    Ok(LivePty {
        pid,
        master: pair.master,
        writer: Mutex::new(writer),
        _child: child,
    })
}

pub fn spawn_captured(
    command: &str,
    args: &[String],
    log_path: &std::path::Path,
    env: &[(&str, &str)],
) -> std::io::Result<u32> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = std::fs::File::create(log_path)?;
    let log_err = log.try_clone()?;
    let child = std::process::Command::new(command)
        .args(args)
        .envs(env.iter().copied())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(log))
        .stderr(std::process::Stdio::from(log_err))
        .spawn()?;
    Ok(child.id())
}

pub fn process_alive(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn terminate(pid: u32) -> std::io::Result<()> {
    if !process_alive(pid) {
        return Ok(());
    }
    signal(pid, "TERM")?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while process_alive(pid) {
        if std::time::Instant::now() >= deadline {
            signal(pid, "KILL")?;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Ok(())
}

fn signal(pid: u32, sig: &str) -> std::io::Result<()> {
    std::process::Command::new("kill")
        .arg(format!("-{sig}"))
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(())
}

fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn spawn_pty_returns_positive_pid_for_running_child() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 1".into()], &[]).unwrap();
        assert!(pid > 0, "spawn must return a child PID; got {pid}");
        assert!(
            process_alive(pid),
            "child {pid} must be alive immediately after spawn"
        );
    }

    #[test]
    fn spawn_pty_survives_parent_handle_drop() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 2".into()], &[]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            process_alive(pid),
            "child {pid} must outlive the spawn handles being dropped"
        );
    }

    #[test]
    fn spawn_captured_propagates_room_role_to_the_child_environment() {
        let log_path = std::env::temp_dir().join(format!(
            "bpmux-room-role-env-{}-{}.log",
            std::process::id(),
            "captured"
        ));
        let _ = std::fs::remove_file(&log_path);
        let pid = spawn_captured(
            "/bin/sh",
            &["-c".into(), "printf 'BPMUX_ROLE=%s' \"$BPMUX_ROLE\"".into()],
            &log_path,
            &[("BPMUX_ROLE", "pm")],
        )
        .unwrap();
        assert!(pid > 0, "captured spawn must report a child PID");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut contents = String::new();
        while std::time::Instant::now() < deadline {
            contents = std::fs::read_to_string(&log_path).unwrap_or_default();
            if contents.contains("BPMUX_ROLE=pm") {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let _ = std::fs::remove_file(&log_path);
        assert!(
            contents.contains("BPMUX_ROLE=pm"),
            "the room-role must reach the spawned child via BPMUX_ROLE so its bridge adopts the room-identity; got {contents:?}"
        );
    }

    #[test]
    fn spawn_pty_retained_reads_child_output_through_the_kept_master() {
        use std::io::Read;
        let live = spawn_pty_retained(
            "/bin/sh",
            &["-c".into(), "printf 'hello-retained-pty'".into()],
            None,
            &[],
        )
        .unwrap();
        assert!(live.pid() > 0, "retained-pty spawn must report a child PID");
        let mut reader = live.reader().unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("hello-retained-pty") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            collected.contains("hello-retained-pty"),
            "the RETAINED master must yield the child's live PTY output (not dropped like spawn_pty); got {collected:?}"
        );
    }

    #[test]
    fn spawn_pty_retained_runs_a_real_shell_in_the_given_cwd_with_term() {
        use std::io::Read;
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
        live.write_input(b"echo TERM=$TERM PWD=$PWD\n").unwrap();
        let mut reader = live.reader().unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
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
    fn resolve_terminal_spawn_defaults_to_a_login_shell_when_no_override() {
        let (command, args) = resolve_terminal_spawn(None, Some("/bin/zsh"));
        assert_eq!(
            command, "/bin/zsh",
            "with no override the terminal spawns the user's $SHELL"
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
            "an explicit BPMUX_ROOM_AGENT override wins over $SHELL"
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
}
