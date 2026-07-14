#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const SYNC_ENGINE_TOKEN: &str = "crate::sync";

const TEST_HARNESS_HOST_FILES: &[&str] = &["test_support.rs", "select.rs"];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read store dir") {
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

fn is_test_harness_host(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| TEST_HARNESS_HOST_FILES.contains(&name))
}

#[test]
fn store_production_files_never_name_the_sync_engine() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/store");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected store source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        if is_test_harness_host(file) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read store file");
        if is_test_source(file, &contents) {
            continue;
        }
        if contents.contains(SYNC_ENGINE_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "the store COMPOSES slices; it never reaches into the synchronization engine. The sync \
         state vocabulary (MutationId / QueuedEvent / PendingWrite / QueuedWrite / SyncStatus / \
         DroppedEvent) and both sync slices live in the source_sync component — import them as \
         source_sync::... or through the store's slice aliases (crate::store::sync, \
         crate::store::sync_health). Sync's store-registered effectors (GithubDrain, \
         OutboxPersister) live in src/sync and register themselves INTO the store; the store \
         never names them back. A store production file naming crate::sync re-grows the \
         store<->sync cycle the source_sync extraction collapsed — flip the import to \
         source_sync:: (vocabulary) or route the effect through a dispatched action (machinery). \
         Files naming crate::sync:\n{}",
        violations.join("\n")
    );
}
