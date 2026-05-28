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

fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn process_is_alive(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    #[test]
    fn spawn_pty_returns_positive_pid_for_running_child() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 1".into()]).unwrap();
        assert!(pid > 0, "spawn must return a child PID; got {pid}");
        assert!(
            process_is_alive(pid),
            "child {pid} must be alive immediately after spawn"
        );
    }

    #[test]
    fn spawn_pty_survives_parent_handle_drop() {
        let pid = spawn_pty("/bin/sh", &["-c".into(), "sleep 2".into()]).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(
            process_is_alive(pid),
            "child {pid} must outlive the spawn handles being dropped"
        );
    }
}
