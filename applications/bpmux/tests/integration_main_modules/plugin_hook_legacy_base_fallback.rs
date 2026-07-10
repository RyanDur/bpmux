#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::os::unix::fs as unix_fs;
use std::path::PathBuf;
use std::process::Command;

fn check_bpmux_script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap()
        .join("claude-plugin/hooks/check-bpmux.sh")
}

#[test]
fn check_bpmux_hook_falls_back_to_the_legacy_sessions_base_before_the_first_migrated_boot() {
    let tmp = tempfile::tempdir().unwrap();
    let sessions_base = tmp.path().join(".bpmux/sessions");
    let legacy_dir = sessions_base.join("legacy-data-dir");
    fs::create_dir_all(&legacy_dir).unwrap();
    unix_fs::symlink(&legacy_dir, sessions_base.join("current")).unwrap();
    let jsonl = r#"{"from":"alice","text":"hello from the legacy layout","ts":"2026-04-03T12:00:00Z"}
"#;
    fs::write(legacy_dir.join("messages-current.jsonl"), jsonl).unwrap();

    let output = Command::new("bash")
        .arg(check_bpmux_script())
        .env("HOME", tmp.path())
        .env_remove("BPMUX_DATA_BASE")
        .output()
        .unwrap();

    assert!(output.status.success(), "script exited non-zero");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bpmux:"),
        "with no data/ yet, the hook must keep reading the legacy sessions/ layout (the hook can fire before the first migrated boot); got: {stdout}"
    );
    assert!(
        stdout.contains("alice"),
        "the legacy-layout message surfaces through the fallback; got: {stdout}"
    );
}
