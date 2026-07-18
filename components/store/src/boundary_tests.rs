#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const SYNC_ENGINE_TYPE_TOKENS: &[&str] = &[
    "SyncOrchestrator",
    "OutboxPersister",
    "OutboxSink",
    "GithubDrain",
];

const TEST_HARNESS_HOST_FILES: &[&str] = &["test_support.rs", "select.rs"];

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read store crate dir") {
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

fn names_a_sync_engine_dependency(manifest_dir: &Path) -> bool {
    let cargo_toml =
        std::fs::read_to_string(manifest_dir.join("Cargo.toml")).expect("read Cargo.toml");
    cargo_toml.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("sync ") || trimmed.starts_with("sync=") || trimmed.starts_with("sync.")
    })
}

#[test]
fn store_crate_never_names_the_app_sync_engine() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !names_a_sync_engine_dependency(&manifest_dir),
        "the store crate's Cargo.toml must never path-dep a crate literally named `sync` — the \
         app's synchronization engine (applications/bpmux/src/sync) is a delivery-tier module, \
         not a workspace component, and the store crate must stay unreachable from it"
    );

    let root = manifest_dir.join("src");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected store crate source files under {}",
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
        let contents = std::fs::read_to_string(file).expect("read store crate file");
        if is_test_source(file, &contents) {
            continue;
        }
        for token in SYNC_ENGINE_TYPE_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "the store crate COMPOSES slices; as its own workspace crate it has no path back to the \
         app's synchronization engine, and must never name its effector types. The sync state \
         vocabulary (MutationId / QueuedEvent / PendingWrite / QueuedWrite / SyncStatus / \
         DroppedEvent / SourceAdapter) lives in the source_sync component and is a legitimate \
         dependency — import it as source_sync::... or through the store's slice aliases \
         (crate::sync, crate::sync_health). The engine's own effectors (GithubDrain, \
         OutboxPersister, OutboxSink) and its SyncOrchestrator register themselves INTO the store \
         from the app side; the store crate never names them back. Violations:\n{}",
        violations.join("\n")
    );
}
