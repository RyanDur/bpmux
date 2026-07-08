#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const FORBIDDEN_TOKENS: &[&str] = &[
    "crate::store",
    "crate::transport",
    "crate::integrations",
    "crate::bridge",
    "crate::sync",
    "crate::product_management::scheduling",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read view dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

fn is_test_source(path: &Path, contents: &str) -> bool {
    path.to_string_lossy().contains("_tests")
        || contents.contains("#[test]")
        || contents.contains("#[tokio::test]")
}

#[test]
fn view_formats_domain_data_only() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/view");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected view source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read view file");
        if is_test_source(file, &contents) {
            continue;
        }
        for token in FORBIDDEN_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "view is the read-model projection: it formats already-selected domain data (product_management types, workflow config, message) into the renderer wire shape and nothing more. It must not reach the redux composition (crate::store), the delivery edge (crate::transport), an integration adapter (crate::integrations), the MCP surface (crate::bridge), the synchronization engine (crate::sync), or the state-projection derivation (crate::product_management::scheduling — the view receives the already-projected column/state, it does not re-derive it). A view file naming one of these is pulling infrastructure or domain derivation into a projection — take the projected value as a function argument instead; the composition/surface selects from the store and hands the view plain domain data. Test projections that construct a concrete codec are exempt. Violations:\n{}",
        violations.join("\n")
    );
}
