#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const FORBIDDEN_VENDOR_TOKENS: &[&str] = &[
    "octocrab",
    "reqwest",
    "crate::github",
    "crate::bridge::tracker",
];

const SELF_FILE: &str = "boundary_tests.rs";

const FORBIDDEN_EDGE_TOKENS: &[&str] = &["crate::storage", "crate::transport"];

const REDUCER_DIR: &str = "src/product_management/story/slice/reducers";

const REDUCER_FORBIDDEN_DECODE_TOKENS: &[&str] = &[
    "adapters::github",
    "gh_label_codec",
    "_from_labels",
    "project_labels",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read product_management dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn product_management_domain_imports_no_vendor_integration() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/product_management");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find product_management source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        for token in FORBIDDEN_VENDOR_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "product_management is the integration-agnostic domain: it depends on the SourceAdapter port, never on a concrete integration mechanism (octocrab, reqwest, crate::github wire client, crate::bridge::tracker). Move the offending code into integrations/<provider>/ (gateway + mapper + adapter) and inject the concrete gateway at the composition root. Violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn product_management_domain_imports_no_infrastructure_edge() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/product_management");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find product_management source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        for token in FORBIDDEN_EDGE_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "product_management is the domain: it does not reach the infrastructure edges (crate::storage persistence, crate::transport delivery) directly. State lives in the store; the synchronization engine (crate::sync) owns persistence and projects off the store; identity/name helpers live in crate::routing. Move the offending code into crate::sync, behind a store-owned port, or to a support module (crate::routing). Violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn board_reducer_does_no_label_decode() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(REDUCER_DIR);
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find board reducer source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let contents = std::fs::read_to_string(file).expect("read reducer file");
        for token in REDUCER_FORBIDDEN_DECODE_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the story slice reducer (story/slice/reducers) is a PURE (state, action) -> state assign: it must not decode GitHub label format. Decoding belongs in the slice middleware (story/slice/middleware.rs), which projects a label set into typed LabelDerivedFields that the reducer assigns. Move the offending decode into the middleware. Violations:\n{}",
        violations.join("\n")
    );
}
