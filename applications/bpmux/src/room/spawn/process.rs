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

pub(crate) fn signal(pid: u32, sig: &str) -> std::io::Result<()> {
    std::process::Command::new("kill")
        .arg(format!("-{sig}"))
        .arg(pid.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    Ok(())
}

pub(crate) fn io_other<E: std::fmt::Display>(e: E) -> std::io::Error {
    std::io::Error::other(e.to_string())
}
