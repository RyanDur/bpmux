//! End-to-end test for the Claude Code token adapter (#179).
//!
//! Shape: behavior test, not unit test. The adapter is a bash/jq
//! script in `claude-plugin/hooks/token-adapter.sh`. This test
//! exercises the REAL script (not a fake), feeding it a fake
//! transcript file and a fake hook JSON payload, and asserts on what
//! the script actually writes to the token log. Then it reads the
//! log back via the `bpmux::session::tokens::read_token_rows` Rust
//! function — so the round-trip proves the script writes in the
//! schema the Rust reader accepts.
//!
//! This is the "adapter contract" test: if a second adapter (Cursor,
//! Codex, ...) lands later, it should be able to produce the same
//! kind of output this test asserts on.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::process::{Command, Stdio};

use bpmux::session::tokens::{TokenRow, read_token_rows};

/// Build a minimal Claude Code transcript jsonl with a single assistant
/// entry carrying usage data. Matches the shape observed in real
/// transcripts under `~/.claude/projects/.../uuid.jsonl`.
fn fake_transcript(uuid: &str, input: u64, cache_read: u64, output: u64) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":"hi"}},"timestamp":"2026-04-11T22:00:00.000Z","uuid":"u1"}}
{{"type":"assistant","uuid":"{uuid}","timestamp":"2026-04-11T22:00:05.000Z","message":{{"model":"claude-opus-4-6","content":[{{"type":"text","text":"hello"}}],"usage":{{"input_tokens":{input},"cache_creation_input_tokens":0,"cache_read_input_tokens":{cache_read},"output_tokens":{output}}}}}}}
"#
    )
}

fn adapter_path() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest)
        .join("claude-plugin")
        .join("hooks")
        .join("token-adapter.sh")
}

fn run_adapter(
    transcript_path: &std::path::Path,
    tokens_dir: &std::path::Path,
    session: &str,
    role: &str,
) {
    let hook_json = format!(
        r#"{{"session_id":"{session}","transcript_path":"{}"}}"#,
        transcript_path.to_string_lossy()
    );
    let mut child = Command::new("bash")
        .arg(adapter_path())
        .env("BPMUX_ROLE", role)
        .env("BPMUX_TOKENS_DIR", tokens_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn adapter");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_json.as_bytes())
        .expect("write hook json");
    let status = child.wait().expect("wait adapter");
    assert!(status.success(), "adapter exited non-zero: {status:?}");
}

#[test]
fn adapter_writes_one_row_matching_the_transcript_usage() {
    let tmp = tempfile::tempdir().unwrap();
    let transcript_path = tmp.path().join("transcript.jsonl");
    std::fs::write(&transcript_path, fake_transcript("uuid-a", 12, 8200, 450)).unwrap();

    let tokens_dir = tmp.path().join("tokens");
    run_adapter(&transcript_path, &tokens_dir, "sess-001", "dev");

    let rows = read_token_rows(&tokens_dir.join("log.jsonl")).unwrap();
    assert_eq!(rows.len(), 1, "expected exactly one row, got: {rows:?}");
    let row = &rows[0];
    assert_eq!(row.role, "dev");
    assert_eq!(row.harness, "claude-code");
    assert_eq!(row.model, "claude-opus-4-6");
    assert_eq!(row.input_tokens, 12);
    assert_eq!(row.cache_read_input_tokens, 8200);
    assert_eq!(row.output_tokens, 450);
    assert_eq!(row.session_id.as_deref(), Some("sess-001"));
}

#[test]
fn adapter_dedupes_when_last_assistant_uuid_has_not_changed() {
    let tmp = tempfile::tempdir().unwrap();
    let transcript_path = tmp.path().join("transcript.jsonl");
    std::fs::write(&transcript_path, fake_transcript("uuid-b", 5, 100, 10)).unwrap();

    let tokens_dir = tmp.path().join("tokens");
    run_adapter(&transcript_path, &tokens_dir, "sess-002", "pm");
    // Second call on unchanged transcript should be a no-op.
    run_adapter(&transcript_path, &tokens_dir, "sess-002", "pm");

    let rows = read_token_rows(&tokens_dir.join("log.jsonl")).unwrap();
    assert_eq!(
        rows.len(),
        1,
        "expected dedup: one row after two adapter runs with same transcript uuid, got {}",
        rows.len()
    );
}

#[test]
fn adapter_emits_new_row_when_transcript_grows_with_new_assistant_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let transcript_path = tmp.path().join("transcript.jsonl");
    std::fs::write(&transcript_path, fake_transcript("uuid-c1", 10, 50, 20)).unwrap();

    let tokens_dir = tmp.path().join("tokens");
    run_adapter(&transcript_path, &tokens_dir, "sess-003", "designer");

    // Append a second assistant entry — simulating the next turn.
    let mut appended = fake_transcript("uuid-c1", 10, 50, 20);
    appended.push_str(&fake_transcript("uuid-c2", 30, 200, 80));
    std::fs::write(&transcript_path, appended).unwrap();

    run_adapter(&transcript_path, &tokens_dir, "sess-003", "designer");

    let rows = read_token_rows(&tokens_dir.join("log.jsonl")).unwrap();
    assert_eq!(
        rows.len(),
        2,
        "expected two rows: one per distinct assistant uuid, got {}",
        rows.len()
    );
    assert_eq!(rows[0].output_tokens, 20);
    assert_eq!(rows[1].output_tokens, 80);
}

#[test]
fn adapter_exits_silently_when_transcript_path_missing() {
    // If hook payload lacks transcript_path or the file doesn't exist,
    // the adapter should exit 0 without creating any log.
    let tmp = tempfile::tempdir().unwrap();
    let tokens_dir = tmp.path().join("tokens");
    let hook_json = r#"{"session_id":"sess-004"}"#;

    let mut child = Command::new("bash")
        .arg(adapter_path())
        .env("BPMUX_ROLE", "dev")
        .env("BPMUX_TOKENS_DIR", &tokens_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn adapter");
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(hook_json.as_bytes())
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());

    // No log file should have been created.
    let log_path = tokens_dir.join("log.jsonl");
    assert!(
        !log_path.exists(),
        "adapter should not create log for missing transcript"
    );
    // Silence unused-import warning if TokenRow is unreferenced in this test.
    let _ = std::marker::PhantomData::<TokenRow>;
}
