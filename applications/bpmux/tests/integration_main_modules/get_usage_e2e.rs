#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use bpmux::telemetry::usage::{Record, save_to};
use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::collections::HashMap;

#[test]
fn get_usage_returns_the_bridges_isolated_usage_file() {
    let bpmux = TestBpmux::start();
    let probe = Record {
        count: 3,
        last_used: "2026-04-16T12:00:00+00:00".to_string(),
    };
    let mut seeded: HashMap<String, Record> = HashMap::new();
    seeded.insert("mcp:seeded_probe".to_string(), probe.clone());
    save_to(&bpmux.usage_path(), &seeded).expect("seed usage.json");

    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;

        let result = dev
            .call_tool("get_usage", serde_json::json!({}))
            .await
            .expect("get_usage should succeed");

        let parsed: HashMap<String, Record> = match &result {
            serde_json::Value::Object(_) => serde_json::from_value(result.clone()),
            serde_json::Value::String(s) => serde_json::from_str(s),
            other => panic!("get_usage returned unexpected shape: {other:?}"),
        }
        .expect("get_usage returns a well-formed usage map");

        assert_eq!(
            parsed.get("mcp:seeded_probe"),
            Some(&probe),
            "get_usage reads the bridge's BPMUX_USAGE_PATH file, returning the seeded record verbatim"
        );
        assert!(
            parsed.contains_key("mcp:get_usage"),
            "get_usage returns the live usage file, which records the dispatched call itself"
        );

        dev.close().await;
    });
}
