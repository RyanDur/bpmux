#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};

fn src_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

const PORT_OWNING_FEATURE_MODULES: &[&str] = &[
    "../../awareness/src",
    "../../message_log/src",
    "../../memory/src",
    "../../source_sync/src",
    "../../user_routing/src",
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
fn domain_sql_lives_only_in_each_crate_db_module() {
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
        if contents.contains("refinery") {
            violations.push(format!(
                "{}: refinery (migrations belong to database-support alone)",
                file.display()
            ));
        }
        let is_mechanism_module = file
            .file_name()
            .is_some_and(|n| n == "db.rs" || n == "data_gateway.rs" || n == "import.rs");
        if !is_mechanism_module && contents.contains("rusqlite") {
            violations.push(format!(
                "{}: rusqlite outside the db/data_gateway/import mechanism modules",
                file.display()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "each domain talks to its own tables through the injected gateway (owner ruling \
         2026-07-25): SQL mechanism is confined to the crate's db module, the port itself \
         stays an abstract interface, and migrations are applied only by database-support. \
         Violations:\n{}",
        violations.join("\n")
    );
}
