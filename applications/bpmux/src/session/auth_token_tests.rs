#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::auth_token::{default_auth_token_path, ensure, eq, generate_and_write, read};
use tempfile::TempDir;

fn tmp() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("auth_token");
    (dir, path)
}

#[test]
fn generate_writes_hex_and_reads_back_identical() {
    let (_dir, path) = tmp();
    let written = generate_and_write(&path).unwrap();
    assert_eq!(written.len(), 64, "32 bytes hex-encoded = 64 chars");
    assert!(
        written.chars().all(|c| c.is_ascii_hexdigit()),
        "expected hex chars, got: {written}"
    );
    let loaded = read(&path).unwrap();
    assert_eq!(loaded, written);
}

#[test]
fn generate_produces_distinct_tokens() {
    let (_dir, path) = tmp();
    let a = generate_and_write(&path).unwrap();
    let b = generate_and_write(&path).unwrap();
    assert_ne!(a, b, "each launch should rotate the secret");
}

#[cfg(unix)]
#[test]
fn generate_writes_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, path) = tmp();
    generate_and_write(&path).unwrap();
    let meta = std::fs::metadata(&path).unwrap();
    let mode = meta.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "auth_token must be owner-read-write only");
}

#[test]
fn ensure_generates_on_missing() {
    let (_dir, path) = tmp();
    assert!(!path.exists());
    let t = ensure(&path).unwrap();
    assert!(path.exists(), "ensure should create the file");
    assert_eq!(t.len(), 64);
}

#[test]
fn ensure_returns_existing_without_overwriting() {
    let (_dir, path) = tmp();
    let first = ensure(&path).unwrap();
    let second = ensure(&path).unwrap();
    assert_eq!(first, second, "ensure is idempotent when file exists");
}

#[test]
fn ensure_regenerates_when_file_is_empty() {
    let (_dir, path) = tmp();
    std::fs::write(&path, "").unwrap();
    let t = ensure(&path).unwrap();
    assert!(!t.is_empty(), "empty file should trigger regenerate");
}

#[test]
fn eq_rejects_mismatched_length() {
    assert!(!eq("abc", "abcd"));
    assert!(!eq("abcd", "abc"));
}

#[test]
fn eq_accepts_exact_match() {
    assert!(eq("deadbeef", "deadbeef"));
}

#[test]
fn eq_rejects_same_length_different_content() {
    assert!(!eq("deadbeef", "deadbeec"));
}

#[test]
fn default_path_returns_some_path() {
    let got = default_auth_token_path();
    assert!(got.is_some(), "expected a default auth token path");
}
