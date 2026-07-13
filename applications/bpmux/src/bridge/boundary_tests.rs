#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const VENDOR_SDK_TOKEN: &str = "octocrab";

const VENDOR_ADAPTER_MARKER: &str = "tracker";

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

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read bridge dir") {
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

#[test]
fn bridge_edge_imports_the_vendor_sdk_only_in_its_tracker_adapter() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/bridge");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected bridge source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        if file.to_string_lossy().contains(VENDOR_ADAPTER_MARKER) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read bridge file");
        if is_test_source(file, &contents) {
            continue;
        }
        if contents.contains(VENDOR_SDK_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "the vendor SDK (octocrab) is confined to the bridge's GitHub tracker adapter (bridge/tracker/**, the anti-corruption home of the concrete client); the rest of the bridge talks to the local bpmux server over its own reqwest RPC transport and consumes the tracker behind the OctocrabStoryTracker/RefreshDriver ports. A bridge file outside tracker/ naming octocrab is leaking the vendor mechanism into the MCP surface — take a port instead and wire the concrete adapter at the composition root. Files naming octocrab outside the tracker adapter:\n{}",
        violations.join("\n")
    );
}

#[test]
fn bridge_surface_reads_no_raw_slice_state() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/bridge");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected bridge source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read bridge file");
        if is_test_source(file, &contents) {
            continue;
        }
        for hit in raw_grain_reads(&contents) {
            violations.push(format!("{}: {hit}", file.display()));
        }
    }

    assert!(
        violations.is_empty(),
        "the bridge is a surface: inside a select closure it passes a whole store grain to a named domain selector (product_management::...(&state.stories)) or clones a grain wholesale (state.shared_history.clone()), never pokes into a grain's collection (state.stories.values(), state.iterations.get(...), state.board.iter()). A raw slice read hard-codes projection logic into the surface that belongs in a domain selector — add the selector in the owning slice and call it. Raw slice reads in the bridge:\n{}",
        violations.join("\n")
    );
}
