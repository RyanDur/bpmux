use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::broadcast;

use super::input_activity::InputActivity;
use super::live_pty::LivePty;
use super::output_reader::spawn_output_reader;
use super::process::io_other;

const PTY_OUTPUT_CAPACITY: usize = 1024;

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
