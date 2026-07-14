#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

const SELF_FILE: &str = "boundary_tests.rs";

const STORE_TOKEN: &str = "crate::store";

const VENDOR_SDK_TOKEN: &str = "octocrab";

const VENDOR_ADAPTER_PATH: &str = "github/tracker";

const BRIDGE_TOKEN: &str = "crate::bridge";

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("read integrations dir") {
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
fn integrations_implement_domain_ports_without_naming_the_store() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/integrations");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected integrations source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read integrations file");
        if is_test_source(file, &contents) {
            continue;
        }
        if contents.contains(STORE_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "integrations are adapters at the hexagonal edge: each one implements a domain port (e.g. crate::ci::slice::CiPollFetch) named at that port's DOMAIN home, never through the store composition kernel or its re-export aliases (store::ci is merely `pub use crate::ci::slice as ci`). A crate::store reference inside an adapter points the dependency arrow at the composition kernel instead of the domain that owns the port it implements — name the domain module directly. No allowlist: integrations must be store-free, period. Violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn integrations_confine_the_vendor_sdk_to_the_github_tracker_adapter() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/integrations");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected integrations source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        if file.to_string_lossy().contains(VENDOR_ADAPTER_PATH) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read integrations file");
        if is_test_source(file, &contents) {
            continue;
        }
        if contents.contains(VENDOR_SDK_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "the vendor SDK (octocrab) is confined to the github integration's tracker adapter (integrations/github/tracker/**, the anti-corruption home of the concrete GitHub client); every other integrations file works with domain types and ports (StoryTracker, LabelCodec, SourceAdapter). A second file naming octocrab opens a second vendor seam that the tracker adapter exists to prevent — route the call through the tracker adapter instead. Files naming octocrab outside the tracker adapter:\n{}",
        violations.join("\n")
    );
}

#[test]
fn integrations_never_name_the_bridge() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/integrations");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    assert!(
        !files.is_empty(),
        "expected integrations source files under {}",
        root.display()
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        if file.file_name().and_then(|n| n.to_str()) == Some(SELF_FILE) {
            continue;
        }
        let contents = std::fs::read_to_string(file).expect("read integrations file");
        if is_test_source(file, &contents) {
            continue;
        }
        if contents.contains(BRIDGE_TOKEN) {
            violations.push(file.display().to_string());
        }
    }

    assert!(
        violations.is_empty(),
        "integrations are adapters at the hexagonal edge: the dependency arrow runs bridge -> integrations through domain ports, never back. An integrations prod file naming crate::bridge re-couples the adapter to the MCP surface it serves — move the shared vocabulary to its domain home (crate::ci, crate::sync, product_management) and name it there. Violations:\n{}",
        violations.join("\n")
    );
}
