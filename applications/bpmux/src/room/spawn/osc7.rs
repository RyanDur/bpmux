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
