#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

const COMPOSED_FEATURE_PATHS: &[&str] = &[
    "crate::awareness",
    "message_log",
    "crate::memory",
    "crate::sync",
    "user_routing::",
];

#[test]
fn storage_composes_feature_ports_only_in_the_super_trait_bound() {
    let mod_rs = src_root().join("storage/mod.rs");
    let text = std::fs::read_to_string(&mod_rs).expect("read storage/mod.rs");

    let trait_start = text
        .find("pub trait Storage")
        .expect("storage/mod.rs must declare `pub trait Storage`");
    let super_trait_bound_ends_at_body_open = trait_start
        + text[trait_start..]
            .find('{')
            .expect("the Storage trait must open a body");
    let outside_the_super_trait_bound = format!(
        "{}{}",
        &text[..trait_start],
        &text[super_trait_bound_ends_at_body_open..]
    );

    let leaked: Vec<&str> = COMPOSED_FEATURE_PATHS
        .iter()
        .copied()
        .filter(|path| outside_the_super_trait_bound.contains(path))
        .collect();

    assert!(
        leaked.is_empty(),
        "storage is the persistence EDGE: features declare the repo ports they need and \
         storage IMPLEMENTS them. The `Storage` super-trait composes those ports in its \
         bound list ONLY; its own body declares nothing but neutral key/value state \
         (load_state/save_state). A composed-feature path outside the bound means a domain \
         persistence method leaked back onto the edge trait — move it behind the owning \
         feature's port (e.g. user_routing::UserRepo). Leaked paths: {}",
        leaked.join(", ")
    );
}

const STORAGE_MECHANISM_TOKENS: &[&str] = &["rusqlite", "refinery"];

const PORT_OWNING_FEATURE_MODULES: &[&str] = &[
    "awareness.rs",
    "../../../components/message_log/src",
    "memory",
    "sync",
    "../../../components/user_routing/src",
];

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        for entry in std::fs::read_dir(path).expect("read port module dir") {
            collect_rs_files(&entry.expect("dir entry").path(), out);
        }
    } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
        out.push(path.to_path_buf());
    }
}

#[test]
fn repo_port_modules_import_no_storage_mechanism() {
    let root = src_root();
    let mut files = Vec::new();
    for module in PORT_OWNING_FEATURE_MODULES {
        collect_rs_files(&root.join(module), &mut files);
    }
    assert!(
        !files.is_empty(),
        "expected to find port-module source files"
    );

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let contents = std::fs::read_to_string(file).expect("read port module file");
        for token in STORAGE_MECHANISM_TOKENS {
            if contents.contains(token) {
                violations.push(format!("{}: {token}", file.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "a repo PORT is an abstract persistence interface: the feature states WHAT it needs \
         (load/save its records) in domain terms, and storage's SQLite/File backends implement \
         it. The SQL mechanism (rusqlite, refinery migrations) lives ONLY in the storage edge, \
         never in a port-owning feature module. Move the offending code into storage/data/. \
         Violations:\n{}",
        violations.join("\n")
    );
}
