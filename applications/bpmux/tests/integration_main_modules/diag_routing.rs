#![allow(clippy::unwrap_used, clippy::expect_used)]

use bpmux::session::diag::log_diag;
use std::fs;

fn read_appended_since(path: &std::path::Path, offset: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut file) = fs::File::open(path) else {
        return String::new();
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return String::new();
    }
    let mut appended = String::new();
    let _ = file.read_to_string(&mut appended);
    appended
}

#[test]
fn log_diag_under_cargo_test_writes_to_tmpdir_not_home() {
    let marker = format!(
        "DIAG-ROUTING-MARKER-{}",
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    );

    let prod_path = std::env::var_os("HOME").map(|h| {
        let mut p = std::path::PathBuf::from(h);
        p.push(".bpmux");
        p.push("diag.log");
        p
    });
    let prod_len_before = prod_path
        .as_ref()
        .and_then(|p| fs::metadata(p).ok())
        .map(|m| m.len())
        .unwrap_or(0);

    log_diag(&marker);

    let test_path = std::env::temp_dir().join("bpmux-test-diag.log");
    let test_after = fs::read_to_string(&test_path).expect("test diag log readable post-write");
    assert!(
        test_after.contains(&marker),
        "marker {marker} must land in {test_path:?}; got tail: {:?}",
        test_after.lines().last()
    );

    if let Some(p) = prod_path.as_ref() {
        let appended = read_appended_since(p, prod_len_before);
        assert!(
            !appended.contains(&marker),
            "production diag log at {p:?} gained the cargo-test marker {marker} — log_diag must never write the prod path under test"
        );
    }
}
