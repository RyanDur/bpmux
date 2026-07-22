use std::io;
use std::path::{Path, PathBuf};

use ring::rand::{SecureRandom, SystemRandom};

const TOKEN_BYTES: usize = 32;

pub fn default_auth_token_path() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("BPMUX_AUTH_TOKEN_PATH") {
        return Some(PathBuf::from(override_path));
    }
    dirs::home_dir().map(|h| h.join(".bpmux").join("auth_token"))
}

pub fn generate_and_write(path: &Path) -> io::Result<String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut bytes = [0u8; TOKEN_BYTES];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| io::Error::other("ring SystemRandom::fill failed"))?;
    let hex = hex_encode(&bytes);
    write_0600(path, &hex)?;
    Ok(hex)
}

pub fn read(path: &Path) -> io::Result<String> {
    std::fs::read_to_string(path).map(|s| s.trim().to_string())
}

pub fn ensure(path: &Path) -> io::Result<String> {
    match read(path) {
        Ok(t) if !t.is_empty() => Ok(t),
        _ => generate_and_write(path),
    }
}

pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(unix)]
fn write_0600(path: &Path, contents: &str) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    f.write_all(contents.as_bytes())
}

#[cfg(not(unix))]
fn write_0600(path: &Path, contents: &str) -> io::Result<()> {
    std::fs::write(path, contents)
}

#[cfg(test)]
mod auth_tests;
