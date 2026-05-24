use crate::session::storage::{FileGlobalStore, GlobalStore, MemoryGlobalStore};
use std::collections::HashMap;
use std::sync::Arc;

fn file_global() -> (Arc<dyn GlobalStore>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let identity_dir = dir.path().join("identity");
    let briefings_dir = dir.path().join("briefings");
    let tokens_log_path = dir.path().join("tokens").join("log.jsonl");
    let usage_path = dir.path().join("usage.json");
    let auth_token_path = dir.path().join("auth_token");
    let global: Arc<dyn GlobalStore> = Arc::new(
        FileGlobalStore::new()
            .with_identity_dir(identity_dir)
            .with_briefings_dir(briefings_dir)
            .with_tokens_log_path(tokens_log_path)
            .with_usage_path(usage_path)
            .with_auth_token_path(auth_token_path),
    );
    (global, dir)
}

fn memory_global() -> (Arc<dyn GlobalStore>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let global: Arc<dyn GlobalStore> = Arc::new(MemoryGlobalStore::new());
    (global, dir)
}

macro_rules! global_contract {
    ($name:ident, $check:expr) => {
        mod $name {
            use super::*;

            #[test]
            fn file_global() {
                let (global, _dir) = super::file_global();
                $check(global.as_ref());
            }

            #[test]
            fn memory_global() {
                let (global, _dir) = super::memory_global();
                $check(global.as_ref());
            }
        }
    };
}

global_contract!(
    load_identity_default_when_unset,
    |global: &dyn GlobalStore| {
        let idx = global.load_identity("dev");
        assert!(idx.values.is_empty());
    }
);

global_contract!(
    save_then_load_identity_roundtrips,
    |global: &dyn GlobalStore| {
        let mut idx = crate::session::identity::IdentityIndex::default();
        idx.values.push(crate::session::identity::IdentityValue {
            name: "test-name".to_string(),
            description: "test-value".to_string(),
            principles: vec![],
        });
        global.save_identity("dev", &idx).unwrap();
        let loaded = global.load_identity("dev");
        assert_eq!(loaded.values.len(), 1);
        assert_eq!(loaded.values[0].name, "test-name");
        assert_eq!(loaded.values[0].description, "test-value");
    }
);

global_contract!(per_role_identity_isolation, |global: &dyn GlobalStore| {
    let mut dev_idx = crate::session::identity::IdentityIndex::default();
    dev_idx
        .values
        .push(crate::session::identity::IdentityValue {
            name: "k".to_string(),
            description: "dev-val".to_string(),
            principles: vec![],
        });
    let mut pm_idx = crate::session::identity::IdentityIndex::default();
    pm_idx.values.push(crate::session::identity::IdentityValue {
        name: "k".to_string(),
        description: "pm-val".to_string(),
        principles: vec![],
    });
    global.save_identity("dev", &dev_idx).unwrap();
    global.save_identity("pm", &pm_idx).unwrap();
    assert_eq!(global.load_identity("dev").values[0].description, "dev-val");
    assert_eq!(global.load_identity("pm").values[0].description, "pm-val");
});

global_contract!(
    load_briefing_empty_when_unset,
    |global: &dyn GlobalStore| {
        let body = global.load_briefing("dev");
        assert!(body.is_empty());
    }
);

global_contract!(
    save_then_load_briefing_roundtrips,
    |global: &dyn GlobalStore| {
        global
            .save_briefing("dev", "# briefing\n\ntest content")
            .unwrap();
        let loaded = global.load_briefing("dev");
        assert_eq!(loaded, "# briefing\n\ntest content");
    }
);

global_contract!(per_role_briefing_isolation, |global: &dyn GlobalStore| {
    global.save_briefing("dev", "dev-content").unwrap();
    global.save_briefing("pm", "pm-content").unwrap();
    assert_eq!(global.load_briefing("dev"), "dev-content");
    assert_eq!(global.load_briefing("pm"), "pm-content");
});

fn sample_token_row(ts: &str, output: u64) -> crate::session::tokens::TokenRow {
    crate::session::tokens::TokenRow {
        ts: ts.to_string(),
        role: "dev".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-opus-4-7".to_string(),
        input_tokens: 100,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 50,
        output_tokens: output,
        tool_call_count: 1,
        turn_duration_ms: None,
        transcript_line: None,
        session_id: None,
    }
}

global_contract!(
    read_token_rows_empty_when_unset,
    |global: &dyn GlobalStore| {
        let rows = global.read_token_rows().unwrap();
        assert!(rows.is_empty());
    }
);

global_contract!(
    append_then_read_token_rows_roundtrips,
    |global: &dyn GlobalStore| {
        let row = sample_token_row("2026-05-11T16:00:00Z", 200);
        global.append_token_row(&row).unwrap();
        let rows = global.read_token_rows().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0], row);
    }
);

global_contract!(
    append_token_rows_preserves_order,
    |global: &dyn GlobalStore| {
        let r1 = sample_token_row("2026-05-11T16:00:00Z", 100);
        let r2 = sample_token_row("2026-05-11T16:01:00Z", 200);
        let r3 = sample_token_row("2026-05-11T16:02:00Z", 300);
        global.append_token_row(&r1).unwrap();
        global.append_token_row(&r2).unwrap();
        global.append_token_row(&r3).unwrap();
        let rows = global.read_token_rows().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].output_tokens, 100);
        assert_eq!(rows[1].output_tokens, 200);
        assert_eq!(rows[2].output_tokens, 300);
    }
);

global_contract!(load_usage_empty_when_unset, |global: &dyn GlobalStore| {
    let map = global.load_usage();
    assert!(map.is_empty());
});

global_contract!(
    save_then_load_usage_roundtrips,
    |global: &dyn GlobalStore| {
        let mut map = HashMap::new();
        map.insert(
            "mcp:start_story".to_string(),
            crate::session::usage::Record {
                count: 5,
                last_used: "2026-05-11T16:00:00Z".to_string(),
            },
        );
        global.save_usage(&map).unwrap();
        let loaded = global.load_usage();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get("mcp:start_story").unwrap().count, 5);
    }
);

global_contract!(
    save_usage_overwrites_previous,
    |global: &dyn GlobalStore| {
        let mut v1 = HashMap::new();
        v1.insert(
            "slash:tokens".to_string(),
            crate::session::usage::Record {
                count: 1,
                last_used: "2026-05-11T16:00:00Z".to_string(),
            },
        );
        global.save_usage(&v1).unwrap();

        let mut v2 = HashMap::new();
        v2.insert(
            "mcp:view_board".to_string(),
            crate::session::usage::Record {
                count: 10,
                last_used: "2026-05-11T16:01:00Z".to_string(),
            },
        );
        global.save_usage(&v2).unwrap();

        let loaded = global.load_usage();
        assert_eq!(loaded.len(), 1);
        assert!(loaded.contains_key("mcp:view_board"));
        assert!(!loaded.contains_key("slash:tokens"));
    }
);

#[test]
fn file_global_ensure_auth_token_generates_and_reads_back() {
    let (global, _dir) = file_global();
    let token = global.ensure_auth_token().unwrap();
    assert!(!token.is_empty());
    assert_eq!(token.len(), 64, "32 bytes hex-encoded");
    let read_back = global.read_auth_token().unwrap();
    assert_eq!(token, read_back);
}

#[test]
fn file_global_ensure_auth_token_is_stable_across_calls() {
    let (global, _dir) = file_global();
    let t1 = global.ensure_auth_token().unwrap();
    let t2 = global.ensure_auth_token().unwrap();
    assert_eq!(t1, t2, "ensure returns existing token when present");
}

#[test]
fn memory_global_auth_token_returns_unsupported() {
    let global = MemoryGlobalStore::new();
    let err = global.ensure_auth_token().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    let err = global.read_auth_token().unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
}
