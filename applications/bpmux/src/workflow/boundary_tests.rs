#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const FORBIDDEN_DOMAIN_TOKEN: &str = "product_management";

const TEST_MODULE_PATH_MARKER: &str = "_tests";

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read workflow dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn workflow_production_imports_no_product_management() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/workflow");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected to find workflow source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.display().to_string().contains(TEST_MODULE_PATH_MARKER) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read source file");
        if contents.contains(FORBIDDEN_DOMAIN_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "crate::workflow is the guard-rails leaf: it defines which states exist and which moves are legal, expressed over its own StageId / State / Transition / Verdict types. It must NOT depend on the product_management domain — that is the workflow<->pm cycle. The domain consumes the workflow (pm reads the config, casts verdicts), never the reverse. If workflow code needs a story concept, the story concept belongs on the workflow side (e.g. StageId, already relocated) or the dependency is inverted (pm passes the value in). Production files reaching product_management:\n{}",
        violations.join("\n")
    );
}
