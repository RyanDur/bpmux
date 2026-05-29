use portable_pty::{CommandBuilder, PtySize, native_pty_system};

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

pub fn spawn_captured(
    command: &str,
    args: &[String],
    log_path: &std::path::Path,
) -> std::io::Result<u32> {
    if let Some(parent) = log_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let log = std::fs::File::create(log_path)?;
    let log_err = log.try_clone()?;
    let child = std::process::Command::new(command)
        .args(args)
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
}
