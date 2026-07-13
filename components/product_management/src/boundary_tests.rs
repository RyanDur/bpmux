#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const REDUCER_DIR: &str = "src/story/slice/reducers";

const REDUCER_FORBIDDEN_DECODE_TOKENS: &[&str] = &[
    "integrations::github",
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
fn product_management_board_module_is_gone() {
    let board = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/board");
    assert!(
        !board.exists(),
        "the board god-module has evaporated — no husk should remain at {}. The board is a projection (crate::view::board + crate::view::velocity), a pair of ports (the SourceAdapter port in crate::sync, the LabelCodec port in product_management::label_codec), and integration adapters (crate::integrations::{{github, in_memory}}). Re-growing product_management/board re-introduces the god-module the restructure retired.",
        board.display()
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
