#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";
const TEST_MODULE_PATH_MARKER: &str = "_tests";

const FORBIDDEN_TOKENS: &[&str] = &["octocrab", "reqwest"];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read board dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn board_slice_is_a_pure_domain_leaf() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find board slice source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let name = file.file_name().and_then(|n| n.to_str());
        if name == Some(SELF_FILE) || file.display().to_string().contains(TEST_MODULE_PATH_MARKER) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        for token in FORBIDDEN_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the board slice owns the instance-local middle placement as a pure domain leaf: a vendor SDK (octocrab, reqwest) never enters the domain. Placement is composed with the story's coarse position by selectors at the read surfaces. Violations:\n{}",
        violations.join("\n")
    );
}
