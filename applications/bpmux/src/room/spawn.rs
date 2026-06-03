use std::sync::Mutex;

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

pub struct LivePty {
    pid: u32,
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Mutex<Box<dyn std::io::Write + Send>>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    _retained_tempdir: Option<tempfile::TempDir>,
}

impl LivePty {
    pub fn pid(&self) -> u32 {
        self.pid
    }

    pub fn attach_tempdir(&mut self, dir: tempfile::TempDir) {
        self._retained_tempdir = Some(dir);
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
    Ok(LivePty {
        pid,
        master: pair.master,
        writer: Mutex::new(writer),
        child,
        _retained_tempdir: None,
    })
}

#[derive(Default)]
pub struct Osc7Integration {
    pub env: Vec<(String, String)>,
    pub args: Vec<String>,
    pub zdotdir: Option<tempfile::TempDir>,
}

pub fn osc7_shell_integration(command: &str) -> std::io::Result<Osc7Integration> {
    let family = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match family {
        "zsh" => zsh_osc7_integration(),
        "bash" => bash_osc7_integration(),
        _ => Ok(Osc7Integration::default()),
    }
}

fn bash_osc7_integration() -> std::io::Result<Osc7Integration> {
    let dir = tempfile::tempdir()?;
    let rcfile = dir.path().join("bpmux-bashrc");
    std::fs::write(&rcfile, bash_rc())?;
    Ok(Osc7Integration {
        env: Vec::new(),
        args: vec!["--rcfile".to_string(), rcfile.to_string_lossy().to_string()],
        zdotdir: Some(dir),
    })
}

fn bash_rc() -> String {
    let mut rc = String::new();
    rc.push_str(
        "if [ -f \"$HOME/.bash_profile\" ]; then . \"$HOME/.bash_profile\"; elif [ -f \"$HOME/.bashrc\" ]; then . \"$HOME/.bashrc\"; fi\n",
    );
    rc.push_str(
        "PROMPT_COMMAND=\"${PROMPT_COMMAND:+$PROMPT_COMMAND; }printf '\\033]7;file://%s%s\\007' \\\"\\${HOSTNAME}\\\" \\\"\\${PWD}\\\"\"\n",
    );
    rc
}

fn zsh_osc7_integration() -> std::io::Result<Osc7Integration> {
    let real = std::env::var("ZDOTDIR")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_else(|| "/".to_string());
    let dir = tempfile::tempdir()?;
    let path = dir.path();
    std::fs::write(path.join(".zshenv"), zsh_source_line(&real, ".zshenv"))?;
    std::fs::write(path.join(".zprofile"), zsh_source_line(&real, ".zprofile"))?;
    std::fs::write(path.join(".zlogin"), zsh_source_line(&real, ".zlogin"))?;
    std::fs::write(path.join(".zshrc"), zsh_rc(&real))?;
    Ok(Osc7Integration {
        env: vec![("ZDOTDIR".to_string(), path.to_string_lossy().to_string())],
        args: Vec::new(),
        zdotdir: Some(dir),
    })
}

fn zsh_source_line(real_zdotdir: &str, file: &str) -> String {
    format!("[ -f \"{real_zdotdir}/{file}\" ] && source \"{real_zdotdir}/{file}\"\n")
}

fn zsh_rc(real_zdotdir: &str) -> String {
    let mut rc = zsh_source_line(real_zdotdir, ".zshrc");
    rc.push_str("autoload -Uz add-zsh-hook\n");
    rc.push_str(
        "_bpmux_emit_osc7() { printf '\\033]7;file://%s%s\\007' \"${HOST}\" \"${PWD}\"; }\n",
    );
    rc.push_str("add-zsh-hook chpwd _bpmux_emit_osc7\n");
    rc.push_str("_bpmux_emit_osc7\n");
    rc
}

pub struct BridgeWiring {
    pub binary: String,
    pub port: u16,
    pub agent_name: String,
}

pub fn terminal_provisioning(
    command: &str,
    bridge: Option<&BridgeWiring>,
) -> std::io::Result<Osc7Integration> {
    let integration = osc7_shell_integration(command)?;
    if let (Some(bridge), Some(dir)) = (bridge, integration.zdotdir.as_ref())
        && let Some(rcfile) = provisioned_rcfile(command, dir.path())
    {
        let bridge_json = dir.path().join("bpmux-bridge.json");
        std::fs::write(&bridge_json, bridge_mcp_config(bridge))?;
        append_agent_wrapper(&rcfile, "claude", &bridge_json)?;
    }
    Ok(integration)
}

fn provisioned_rcfile(command: &str, tempdir: &std::path::Path) -> Option<std::path::PathBuf> {
    let family = std::path::Path::new(command)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    match family {
        "bash" => Some(tempdir.join("bpmux-bashrc")),
        "zsh" => Some(tempdir.join(".zshrc")),
        _ => None,
    }
}

fn bridge_mcp_config(bridge: &BridgeWiring) -> String {
    serde_json::json!({
        "mcpServers": {
            "bpmux-bridge": {
                "type": "stdio",
                "command": bridge.binary,
                "args": [
                    "bridge",
                    "--port",
                    bridge.port.to_string(),
                    "--name",
                    bridge.agent_name,
                ],
            }
        }
    })
    .to_string()
}

fn append_agent_wrapper(
    rcfile: &std::path::Path,
    agent: &str,
    config: &std::path::Path,
) -> std::io::Result<()> {
    use std::io::Write;
    let config = config.to_string_lossy();
    let block = format!("{agent}() {{ command {agent} --mcp-config=\"{config}\" \"$@\"; }}\n");
    let mut file = std::fs::OpenOptions::new().append(true).open(rcfile)?;
    file.write_all(block.as_bytes())
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
    fn live_pty_resize_propagates_the_winsize_to_the_shell_tty() {
        use std::io::Read;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let live = spawn_pty_retained(&shell, &[], None, &[("TERM", "xterm-256color")]).unwrap();
        live.resize(40, 120).unwrap();
        live.write_input(b"stty size\n").unwrap();
        let mut reader = live.reader().unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("40 120") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
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

    #[test]
    fn spawned_shell_emits_osc7_carrying_the_new_cwd_after_cd() {
        use std::io::Read;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let integration = osc7_shell_integration(&shell).unwrap();
        let target =
            std::env::temp_dir().join(format!("bpmux-osc7-{}-{}", std::process::id(), "livecwd"));
        std::fs::create_dir_all(&target).unwrap();
        let marker = target.file_name().unwrap().to_string_lossy().to_string();
        let mut env: Vec<(&str, &str)> = vec![("TERM", "xterm-256color")];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained(&shell, &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        live.write_input(format!("cd {}\n", target.display()).as_bytes())
            .unwrap();
        let mut reader = live.reader().unwrap();
        let mut collected = String::new();
        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    collected.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if collected.contains("]7;file://") && collected.contains(&marker) {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        let _ = std::fs::remove_dir_all(&target);
        assert!(
            collected.contains("]7;file://") && collected.contains(&marker),
            "the spawned $SHELL must emit OSC-7 (ESC]7;file://…<newdir>) on cd — the native mechanism the terminal header reads to follow cwd; got {collected:?}"
        );
    }

    #[test]
    fn bash_osc7_appends_to_a_user_prompt_command_without_clobbering_it() {
        use std::io::Read;
        let bash = "/bin/bash";
        if !std::path::Path::new(bash).exists() {
            return;
        }
        let home = tempfile::tempdir().unwrap();
        std::fs::write(
            home.path().join(".bashrc"),
            "PROMPT_COMMAND='echo BPMUX_USER_PC_MARKER'\n",
        )
        .unwrap();
        let integration = osc7_shell_integration(bash).unwrap();
        let target = home.path().join("livecwd-target");
        std::fs::create_dir_all(&target).unwrap();
        let marker = target.file_name().unwrap().to_string_lossy().to_string();
        let home_str = home.path().to_string_lossy().to_string();
        let mut env: Vec<(&str, &str)> =
            vec![("TERM", "xterm-256color"), ("HOME", home_str.as_str())];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained(bash, &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        live.write_input(format!("cd {}\n", target.display()).as_bytes())
            .unwrap();
        let reader = live.reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let reader_handle = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                    let osc7_ok = collected.contains("]7;file://") && collected.contains(&marker);
                    if osc7_ok && collected.contains("BPMUX_USER_PC_MARKER") {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        live.close_and_reap();
        drop(rx);
        let _ = reader_handle.join();
        assert!(
            collected.contains("]7;file://") && collected.contains(&marker),
            "bash must STILL emit OSC-7 live-cwd on cd when the user's .bashrc set its own PROMPT_COMMAND — our append must survive the user's rc, not be clobbered; got {collected:?}"
        );
        assert!(
            collected.contains("BPMUX_USER_PC_MARKER"),
            "the user's PROMPT_COMMAND must STILL run — our OSC-7 append must not clobber it (non-destructive, both directions); got {collected:?}"
        );
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

    #[test]
    fn provisioned_bash_wraps_claude_with_mcp_config_and_preserves_user_args() {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        let bash = "/bin/bash";
        if !std::path::Path::new(bash).exists() {
            return;
        }
        let home = tempfile::tempdir().unwrap();
        let bindir = home.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        let stub = bindir.join("claude");
        std::fs::write(&stub, "#!/bin/sh\nprintf 'STUBCLAUDE[%s]\\n' \"$*\"\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();

        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let integration = terminal_provisioning(bash, Some(&bridge)).unwrap();

        let home_str = home.path().to_string_lossy().to_string();
        let bindir_str = bindir.to_string_lossy().to_string();
        let base_path = std::env::var("PATH").unwrap_or_default();
        let path_str = format!("{bindir_str}:{base_path}");
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("HOME", home_str.as_str()),
            ("PATH", path_str.as_str()),
        ];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained(bash, &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        live.write_input(b"claude hello world\n").unwrap();
        let reader = live.reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let reader_handle = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                    if collected.contains("STUBCLAUDE[") && collected.contains("hello world") {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        live.close_and_reap();
        drop(rx);
        let _ = reader_handle.join();
        assert!(
            collected.contains("--mcp-config="),
            "the provisioned shell must wrap claude to invoke the real binary with --mcp-config= (=form so the variadic flag binds one value, not the user prompt); got {collected:?}"
        );
        assert!(
            collected.contains("hello world"),
            "the wrapper must PRESERVE the user's args end-to-end (claude hello world reaches the real claude as the prompt, not swallowed into the variadic config flag); got {collected:?}"
        );
    }

    fn spawn_provisioned_bash_with_stub(
        bridge: Option<&BridgeWiring>,
    ) -> (LivePty, tempfile::TempDir) {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let bindir = home.path().join("bin");
        std::fs::create_dir_all(&bindir).unwrap();
        let stub = bindir.join("claude");
        std::fs::write(&stub, "#!/bin/sh\nprintf 'STUBCLAUDE[%s]\\n' \"$*\"\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let integration = terminal_provisioning("/bin/bash", bridge).unwrap();
        let home_str = home.path().to_string_lossy().to_string();
        let base_path = std::env::var("PATH").unwrap_or_default();
        let path_str = format!("{}:{base_path}", bindir.to_string_lossy());
        let mut env: Vec<(&str, &str)> = vec![
            ("TERM", "xterm-256color"),
            ("HOME", home_str.as_str()),
            ("PATH", path_str.as_str()),
        ];
        for (key, value) in &integration.env {
            env.push((key.as_str(), value.as_str()));
        }
        let mut live = spawn_pty_retained("/bin/bash", &integration.args, None, &env).unwrap();
        if let Some(dir) = integration.zdotdir {
            live.attach_tempdir(dir);
        }
        (live, home)
    }

    fn collect_until(live: LivePty, input: &[u8], until: &str, secs: u64) -> String {
        use std::io::Read;
        live.write_input(input).unwrap();
        let reader = live.reader().unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let handle = std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 1024];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(secs);
        while std::time::Instant::now() < deadline {
            match rx.recv_timeout(std::time::Duration::from_millis(200)) {
                Ok(chunk) => {
                    collected.push_str(&String::from_utf8_lossy(&chunk));
                    if collected.contains(until) {
                        break;
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        live.close_and_reap();
        drop(rx);
        let _ = handle.join();
        collected
    }

    #[test]
    fn unprovisioned_bash_gives_claude_no_bridge() {
        if !std::path::Path::new("/bin/bash").exists() {
            return;
        }
        let (live, _home) = spawn_provisioned_bash_with_stub(None);
        let collected = collect_until(live, b"claude probe\n", "STUBCLAUDE[", 8);
        assert!(
            collected.contains("STUBCLAUDE["),
            "the stub claude must run so the probe is meaningful; got {collected:?}"
        );
        assert!(
            !collected.contains("--mcp-config="),
            "an UNPROVISIONED shell (bridge=None) must give claude NO bridge — proves the wrapper did the work, not an ambient config (honest pre-state discriminator); got {collected:?}"
        );
    }

    #[test]
    fn provisioned_bash_leaves_non_claude_commands_unaffected() {
        if !std::path::Path::new("/bin/bash").exists() {
            return;
        }
        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let (live, _home) = spawn_provisioned_bash_with_stub(Some(&bridge));
        let collected = collect_until(live, b"echo PLAINMARKER_4731\n", "PLAINMARKER_4731", 8);
        assert!(
            collected.contains("PLAINMARKER_4731"),
            "the provisioned terminal must stay a generic shell — a NON-claude command runs unaffected (the agent-wrapper is inert unless claude is launched); got {collected:?}"
        );
    }

    #[test]
    fn bridge_mcp_config_declares_the_session_bridge_with_a_legible_name() {
        let bridge = BridgeWiring {
            binary: "/opt/bin/bpmux".to_string(),
            port: 4117,
            agent_name: "agent-term1".to_string(),
        };
        let cfg: serde_json::Value = serde_json::from_str(&bridge_mcp_config(&bridge)).unwrap();
        let server = &cfg["mcpServers"]["bpmux-bridge"];
        assert_eq!(
            server["command"], "/opt/bin/bpmux",
            "the injected config must point at THIS running bpmux binary so the agent reconnects to the live session"
        );
        assert_eq!(
            server["args"],
            serde_json::json!(["bridge", "--port", "4117", "--name", "agent-term1"]),
            "the bridge args carry the session port + a legible per-terminal --name (else identity collides across N terminals)"
        );
        assert_eq!(server["type"], "stdio");
    }
}
