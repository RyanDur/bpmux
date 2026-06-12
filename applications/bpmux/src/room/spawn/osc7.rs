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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::room::spawn::spawn_pty_retained;

    #[test]
    fn spawned_shell_emits_osc7_carrying_the_new_cwd_after_cd() {
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
        let mut rx = live.subscribe();
        live.write_input(format!("cd {}\n", target.display()).as_bytes())
            .unwrap();
        let mut collected = String::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
        while std::time::Instant::now() < deadline {
            match rx.blocking_recv() {
                Ok(bytes) if bytes.is_empty() => break,
                Ok(bytes) => {
                    collected.push_str(&String::from_utf8_lossy(&bytes));
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
        let mut subscriber = live.subscribe();
        live.write_input(format!("cd {}\n", target.display()).as_bytes())
            .unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        let reader_handle = std::thread::spawn(move || {
            while let Ok(bytes) = subscriber.blocking_recv() {
                if bytes.is_empty() || tx.send(bytes).is_err() {
                    break;
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
}
