#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn migrations_are_forward_only_versioned() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../databases/conversation-db/migrations");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("migrations dir must exist") {
        let name = entry.unwrap().file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        count += 1;
        let versioned = name
            .strip_prefix('V')
            .and_then(|rest| rest.split_once("__"))
            .is_some_and(|(version, _)| {
                !version.is_empty() && version.chars().all(|c| c.is_ascii_digit())
            });
        let migration_ext = name.ends_with(".sql") || name.ends_with(".rs");
        assert!(
            versioned && migration_ext,
            "migration `{name}` must be a forward versioned migration (V{{N}}__name.sql); refinery has no rollback/down — undo via a NEW V migration, never a U/down/rollback file"
        );
    }
    assert!(count > 0, "expected at least the V1 migration in {dir:?}");
}

#[test]
fn migrations_hold_schema_only_never_data() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../databases/conversation-db/migrations");
    for entry in std::fs::read_dir(&dir).expect("migrations dir must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("sql") {
            continue;
        }
        let sql = std::fs::read_to_string(&path).unwrap().to_uppercase();
        for line in sql.lines() {
            let first_word = line.split_whitespace().next().unwrap_or("");
            for dml in ["INSERT", "UPDATE", "DELETE", "REPLACE"] {
                assert!(
                    first_word != dml,
                    "{path:?} starts a {dml} statement — migrations hold schema only (owner \
                     ruling 2026-07-26); data transforms are boot-time maintenance code, \
                     flag-guarded through meta, like the jsonl inheritance"
                );
            }
        }
    }
}
