#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const ALLOWED_CRATE_PATHS: &[&str] = &["crate::message", "crate::ci_link"];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read message dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn message_is_a_self_contained_wire_leaf() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/message");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find message source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        for (line_no, line) in contents.lines().enumerate() {
            let mut stripped = line.to_string();
            for allowed in ALLOWED_CRATE_PATHS {
                stripped = stripped.replace(allowed, "");
            }
            if stripped.contains("crate::") {
                violations.push(format!(
                    "{}:{}: {}",
                    file.display(),
                    line_no + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "message is the shared wire vocabulary — a support LEAF. It imports nothing except its own submodules and crate::ci_link (the CI-actions-url support leaf). Features convert INTO its wire types via From impls they own (see impl From<CiState> for CiStateWire in ci/mod.rs, and From<RosterRole>/From<Presence> in roster/mod.rs); a feature type belongs here only as a byte-identical wire mirror, never as a direct import. Violations:\n{}",
        violations.join("\n")
    );
}
