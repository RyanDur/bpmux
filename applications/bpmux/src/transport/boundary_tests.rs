#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn transport_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/transport")
}

const COMPOSITION_ROOT_PATH: &str = "crate::server";

const THIS_TEST_FILE: &str = "boundary_tests.rs";

const GRAIN_FIELDS: &[&str] = &[
    "stories",
    "iterations",
    "epics",
    "outcomes",
    "board",
    "reactions",
    "shared_history",
    "local_history",
];

fn is_test_source(path: &Path, contents: &str) -> bool {
    path.to_string_lossy().contains("_tests")
        || contents.contains("#[test]")
        || contents.contains("#[tokio::test]")
}

fn raw_grain_reads(contents: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for line in contents.lines() {
        for field in GRAIN_FIELDS {
            let poke = format!("state.{field}.");
            let grain_clone = format!("state.{field}.clone");
            if line.contains(&poke) && !line.contains(&grain_clone) {
                hits.push(line.trim().to_string());
            }
        }
    }
    hits
}

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

#[test]
fn transport_surface_reads_no_raw_slice_state() {
    let mut files = Vec::new();
    collect_rs_files(&transport_root(), &mut files);
    assert!(!files.is_empty(), "expected transport source files");

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let contents = std::fs::read_to_string(file).expect("read transport file");
        if is_test_source(file, &contents) {
            continue;
        }
        for hit in raw_grain_reads(&contents) {
            violations.push(format!("{}: {hit}", file.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "transport is a surface: inside a select closure it passes a whole store grain to a \
         named domain selector or clones a grain wholesale, never pokes into a grain's \
         collection (state.stories.values(), state.iterations.get(...), state.board.iter()). \
         A raw slice read hard-codes projection logic into the delivery edge that belongs in a \
         domain selector — add the selector in the owning slice and call it. Raw slice reads \
         in transport:\n{}",
        violations.join("\n")
    );
}
