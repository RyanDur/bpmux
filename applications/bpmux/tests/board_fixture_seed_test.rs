//! #551: bridge --board-fixture flag was a no-op (run_bridge signature
//! had `_board_fixture: Option<String>` with underscore-prefix
//! indicating ignored param). Post-#516 canonical-store architecture
//! moved the source-of-truth to the electron-server side; the bridge
//! is a thin RPC client. Fixture-loading needs to happen on the
//! canonical-store side, not on the bridge.
//!
//! This test verifies the fix: when a fixture is provided to TestBpmux
//! via `BPMUX_BOARD_FIXTURE` env var, the spawned bpmux server's
//! InMemorySourceAdapter loads stories + bodies from the fixture so
//! get_story / view_board surface the fixture content.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod harness;

use std::io::Write;
use std::time::Duration;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;

#[test]
fn bpmux_loads_fixture_stories_with_bodies_via_env_var() {
    let mut fixture = tempfile::NamedTempFile::new().expect("fixture file");
    fixture
        .write_all(
            format!(
                r###"[{{"number":777,"title":"Fixture-seed test","state":"OPEN","labels":[{{"name":"ready"}}],"body":"## Acceptance test path\n\n{}/tests/harness/story_helpers.rs\n"}}]"###,
                env!("CARGO_MANIFEST_DIR")
            )
            .as_bytes(),
        )
        .expect("write fixture");
    let fixture_path = fixture.path().to_str().expect("utf-8 path").to_string();

    let bpmux = TestBpmux::start_with_fixture(&fixture_path);
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let story = dev
            .call_tool("get_story", serde_json::json!({"number": 777}))
            .await
            .expect("get_story should succeed against fixture-seeded story");
        let story_text = story.as_str().map(String::from).unwrap_or_else(|| story.to_string());
        assert!(
            story_text.contains("Fixture-seed test"),
            "get_story surfaces fixture title; got:\n{story_text}"
        );
        assert!(
            story_text.contains("Acceptance test path"),
            "get_story surfaces fixture body (#551 — bodies must reach InMemorySourceAdapter); got:\n{story_text}"
        );
    });
}
