use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use tokio::sync::watch;

const CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn a task that watches the running binary for changes.
/// Returns a receiver that signals `true` when a reload is needed.
pub fn spawn_watcher() -> watch::Receiver<bool> {
    let (tx, rx) = watch::channel(false);

    let binary_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return rx,
    };

    let initial_mtime = file_mtime(&binary_path);

    tokio::spawn(async move {
        let mut last_mtime = initial_mtime;
        let mut interval = tokio::time::interval(CHECK_INTERVAL);

        loop {
            interval.tick().await;
            let current_mtime = file_mtime(&binary_path);
            if current_mtime != last_mtime && current_mtime.is_some() {
                let _ = tx.send(true);
                break;
            }
            last_mtime = current_mtime;
        }
    });

    rx
}

/// Re-launch the current binary with the same arguments.
/// Uses Unix exec to replace the process in place.
pub fn relaunch() -> anyhow::Result<()> {
    let binary = std::env::current_exe()?;
    let args: Vec<String> = std::env::args().collect();

    eprintln!("bpmux: binary changed, reloading...");

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // exec replaces the process image — safe, no shell involved
        let err = std::process::Command::new(binary).args(&args[1..]).exec();
        anyhow::bail!("relaunch failed: {err}");
    }

    #[cfg(not(unix))]
    {
        anyhow::bail!("hot reload not supported on this platform");
    }
}

fn file_mtime(path: &PathBuf) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}
