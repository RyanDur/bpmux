#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const STORE_BINDING_TOKEN: &str = "crate::store::";

const SLICE_READER_FILE: &str = "src/sync/slice_readers.rs";

const READER_GRAIN_FIELDS: &[&str] = &[
    "roster",
    "reactions",
    "ci",
    "outcomes",
    "iterations",
    "epics",
    "stories",
    "board",
    "shared_history",
    "local_history",
    "sync",
];

const BINDING_SEAM_FILES: &[&str] = &[
    "driver.rs",
    "driver_ops.rs",
    "dispatch.rs",
    "slice_readers.rs",
    "ingest.rs",
    "backfill.rs",
    "mod.rs",
    "drain/mod.rs",
];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read sync dir") {
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
fn sync_names_the_store_only_through_its_binding_seam() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/sync");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected sync source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read sync file");
        if is_test_source(file, &contents) {
            continue;
        }
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");
        if BINDING_SEAM_FILES.contains(&rel.as_ref()) {
            continue;
        }
        if contents.contains(STORE_BINDING_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "the synchronization engine reaches the redux store ONLY through its store-binding seam: driver.rs / driver_ops.rs receive the bound Select and the single Dispatch (bind_select / bind_dispatch), slice_readers.rs reads domain selectors through the bound Select, and dispatch.rs / ingest.rs / backfill.rs / drain/mod.rs / mod.rs carry and emit crate::store::AppAction into that store. Any OTHER sync file naming crate::store:: is deepening the coupling — route reads through a slice_reader and writes through dispatch_board on the Orchestrator's bound handle, never by naming the store composition directly. Files reaching crate::store:: outside the binding seam:\n{}",
        violations.join("\n")
    );
}

fn raw_grain_reads(contents: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for line in contents.lines() {
        for field in READER_GRAIN_FIELDS {
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
fn slice_readers_reach_grains_only_through_named_selectors() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SLICE_READER_FILE);
    let contents = std::fs::read_to_string(&file).expect("read slice_readers source");
    let violations = raw_grain_reads(&contents);
    assert!(
        violations.is_empty(),
        "slice_readers.rs is the designated store-read seam: inside each select closure it hands a whole grain to a named domain selector (story::slice::get(&state.stories, id), crate::board::fine_stage(story, &state.board)) and never pokes into a grain's collection directly (state.stories.is_empty(), state.iterations.get(...)). A raw grain read here is projection logic leaking into the read seam that belongs in the owning slice's selector — add the selector and call it. Raw grain reads in slice_readers.rs:\n{}",
        violations.join("\n")
    );
}
