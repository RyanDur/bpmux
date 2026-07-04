#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn transport_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport")
}

const COMPOSITION_ROOT_PATH: &str = "crate::server";

const THIS_TEST_FILE: &str = "boundary_tests.rs";

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("read transport dir") {
            collect_rs_files(&entry.expect("dir entry").path(), out);
        }
    } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
        && path.file_name().and_then(|n| n.to_str()) != Some(THIS_TEST_FILE)
    {
        out.push(path.to_path_buf());
    }
}

#[test]
fn transport_edge_names_no_composition_root() {
    let mut files = Vec::new();
    collect_rs_files(&transport_root(), &mut files);
    assert!(!files.is_empty(), "expected transport source files");

    let reaching: Vec<String> = files
        .iter()
        .filter(|file| {
            std::fs::read_to_string(file)
                .expect("read transport file")
                .contains(COMPOSITION_ROOT_PATH)
        })
        .map(|file| file.display().to_string())
        .collect();

    assert!(
        reaching.is_empty(),
        "transport is a delivery EDGE: it adapts protocol/store ports, and the server \
         (composition root) wires transport — never the reverse. A crate::server reference \
         means an edge reached up into the root for a type it should get from a support or \
         feature component (the OrchestratorHandle case: home it in crate::sync beside \
         RefreshDriver). Files reaching crate::server:\n{}",
        reaching.join("\n")
    );
}
