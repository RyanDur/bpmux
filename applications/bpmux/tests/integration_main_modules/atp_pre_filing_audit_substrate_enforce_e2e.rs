//! E2E acceptance-test for #1412 — ATP pre-filing-audit substrate
//! gate. Sister to #1362 (post-pickup desk-check ATP-substantive
//! gate); this is the PREVENTIVE create_story-time + edit_story-time
//! validator.
//!
//! Five Gherkin scenarios from #1412's body:
//!   1. ATP-path exists + substantive → 200 (create_story succeeds)
//!   2. ATP-path missing OR stub → 409 with kind="atp-not-substantive"
//!   3. edit_story changing ATP-path re-validates (passes OR 409)
//!   4. Chore-class stories exempt (no ATP-gate fires)
//!   5. Substrate-tests cover positive + 3 negative + planted-violation
//!      seeds (covered via the per-scenario tests below + unit-tests
//!      in atp_substantive_gate::tests)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use std::time::Duration;

#[test]
fn create_story_with_substantive_atp_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let atp = format!(
            "{}/tests/harness/story_helpers.rs",
            env!("CARGO_MANIFEST_DIR")
        );
        let result = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] substantive-atp-passes",
                    "user_job": "When PM files a story with a real ATP, create_story succeeds.",
                    "observation_falsifier": "reject if substantive ATP blocks create_story",
                    "acceptance_test_path": atp
                }),
            )
            .await
            .expect("create_story with substantive ATP should succeed");

        let msg = result.as_str().unwrap_or("");
        assert!(
            msg.contains("story created:"),
            "create_story result names success; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn create_story_with_missing_atp_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let err = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] missing-atp",
                    "user_job": "When PM cites a non-existent ATP, gate refuses pre-filing.",
                    "observation_falsifier": "reject if non-existent ATP slips past filing-gate",
                    "acceptance_test_path": "applications/never/exists/at_all.rs"
                }),
            )
            .await
            .expect_err("missing ATP must be refused at create_story-time");

        assert!(
            err.contains("\"kind\":\"atp-not-substantive\""),
            "structured kind cited; got {err}"
        );
        assert!(
            err.contains("applications/never/exists/at_all.rs"),
            "path cited in error; got {err}"
        );

        pm.close().await;
    });
}

#[test]
fn create_story_with_stub_atp_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let tmp = tempfile::NamedTempFile::with_suffix(".rs").expect("tempfile");
        std::fs::write(
            tmp.path(),
            "// stub test file — no assertion markers\nfn placeholder() {}\n",
        )
        .expect("write stub");
        let stub_path = tmp.path().to_string_lossy().to_string();

        let err = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] stub-atp-refused",
                    "user_job": "When PM cites a stub ATP, gate refuses pre-filing.",
                    "observation_falsifier": "reject if stub ATP slips past filing-gate",
                    "acceptance_test_path": stub_path
                }),
            )
            .await
            .expect_err("stub ATP must be refused at create_story-time");

        assert!(
            err.contains("\"kind\":\"atp-not-substantive\""),
            "structured kind cited; got {err}"
        );
    });
}

#[test]
fn create_chore_story_without_atp_succeeds() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let result = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Chore] builder-substrate-no-atp",
                    "user_job": "When PM files a Chore, no ATP-gate fires (builder-substrate per #1346).",
                    "observation_falsifier": "reject if Chore-class isn't exempted from ATP-gate"
                }),
            )
            .await
            .expect("Chore without ATP should succeed");

        let msg = result.as_str().unwrap_or("");
        assert!(
            msg.contains("story created:"),
            "Chore creates cleanly; got {msg}"
        );

        pm.close().await;
    });
}

#[test]
fn edit_story_changing_atp_to_missing_path_returns_structured_409() {
    let bpmux = TestBpmux::start();
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        let atp = format!(
            "{}/tests/harness/story_helpers.rs",
            env!("CARGO_MANIFEST_DIR")
        );
        let create = pm
            .call_tool(
                "create_story",
                serde_json::json!({
                    "title": "[Feature] edit-to-bad-atp",
                    "user_job": "When PM edits ATP-path to a missing file, gate refuses the edit.",
                    "observation_falsifier": "reject if edit_story bypasses ATP-gate",
                    "acceptance_test_path": atp
                }),
            )
            .await
            .expect("create with substantive ATP");
        let create_msg = create.as_str().unwrap_or("").to_string();
        let id = harness::story_helpers::extract_story_id(&create_msg);

        let bad_body =
            "## User job\n\nstub\n\n## Observation falsifier\n\nreject\n\n## Acceptance test path\n\n`applications/never/exists/edit_target.rs`\n";

        let err = pm
            .call_tool(
                "edit_story",
                serde_json::json!({"id": id, "body": bad_body}),
            )
            .await
            .expect_err("edit_story with missing ATP path must be refused");

        assert!(
            err.contains("\"kind\":\"atp-not-substantive\""),
            "edit_story refusal cites structured kind; got {err}"
        );
        assert!(
            err.contains("applications/never/exists/edit_target.rs"),
            "edit_story refusal cites new bad path; got {err}"
        );

        pm.close().await;
    });
}
