#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "order_state_pin_tests.rs";

const ORDER_FIELD_MARKERS: &[&str] = &[
    "ready_order:",
    ".ready_order",
    "\"ready_order\"",
    "iteration_slot:",
    ".iteration_slot",
    "\"iteration_slot\"",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read src dir") {
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
fn no_order_or_position_field_lives_in_production() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        if is_test_source(file, &contents) {
            continue;
        }
        for marker in ORDER_FIELD_MARKERS {
            if contents.contains(marker) {
                violations.push(format!("{}: {marker}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "containers own order, the story never does (Ryan, 2026-07-07): no production type carries a ready_order or iteration_slot field (declaration, access, or serde key). Ordering lives on the container (the ready queue, iteration membership). A field-shaped occurrence in production is order state creeping back onto an entity — put it on the container. Order/position fields in production:\n{}",
        violations.join("\n")
    );
}
