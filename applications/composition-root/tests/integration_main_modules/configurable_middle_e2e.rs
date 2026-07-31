#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::harness;

use harness::bridge::TestBridge;
use harness::pty::TestBpmux;
use harness::story_helpers::{create_target_story, ensure_current_iteration};
use std::time::Duration;

const THREE_STAGE_WORKFLOW: &str = r#"
[[role]]
name = "owner"
purposes = ["owner"]

[[role]]
name = "pm"
purposes = ["pm"]

[[role]]
name = "designer"
purposes = ["designer", "iteration-reviewer"]

[[role]]
name = "dev"
purposes = ["dev", "iteration-builder"]

[[section]]
id = "pre-work"

[[section]]
id = "in-work"

[[section]]
id = "post-work"

[[state]]
id = "icebox"
section = "pre-work"

[[state]]
id = "ready"
section = "pre-work"

[[state]]
id = "build"
section = "in-work"
judges = ["dev"]

[[state]]
id = "review"
section = "in-work"
judges = ["designer"]

[[state]]
id = "harden"
section = "in-work"

[[state]]
id = "acceptance"
section = "post-work"

[[state]]
id = "accepted"
section = "post-work"

[[state]]
id = "retired"
section = "post-work"

[[transition]]
id = "pickup"
from = "ready"
to = "build"
actor_roles = ["dev"]

[[transition]]
id = "reject"
from = "acceptance"
to = "ready"
actor_roles = ["pm"]

[[transition]]
id = "accept"
from = "acceptance"
to = "accepted"
actor_roles = ["pm"]

[[transition]]
id = "close"
from = "any"
to = "retired"
actor_roles = ["pm"]

[[transition]]
id = "reorder"
from = "any"
to = "any"
actor_roles = ["pm"]

[[transition]]
id = "move-with-position"
from = "any"
to = "any"
actor_roles = ["pm"]

[[transition]]
id = "add-to-iteration"
from = "any"
to = "any"
actor_roles = ["pm"]
"#;

fn project_root_with(workflow_toml: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().expect("temp project root");
    let bpmux_dir = dir.path().join(".bpmux");
    std::fs::create_dir_all(&bpmux_dir).expect("mkdir .bpmux");
    std::fs::write(bpmux_dir.join("workflow.toml"), workflow_toml).expect("write workflow.toml");
    dir
}

fn pass_args(id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "verified_observation": "drove this stage's outcome end-to-end in the running app"
    })
}

#[expect(
    clippy::too_many_lines,
    reason = "an end-to-end journey pinned as one narrative — splitting it into helpers would hide the arc the test documents"
)]
#[test]
fn a_three_stage_middle_drives_end_to_end_with_per_stage_judges() {
    let root = project_root_with(THREE_STAGE_WORKFLOW);
    let bpmux = TestBpmux::start_with_envs(
        "human",
        &[
            ("BPMUX_TEST_MODE", "1"),
            ("BPMUX_PROJECT_ROOT", root.path().to_str().unwrap()),
        ],
    );
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        let designer = TestBridge::connect(&bpmux, "designer").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let board = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board renders");
        let board_text = board.as_str().unwrap_or_default().to_string();
        for header in ["BUILD", "REVIEW", "HARDEN"] {
            assert!(
                board_text.contains(header),
                "the board renders the CONFIGURED middle column {header}; got:\n{board_text}"
            );
        }
        assert!(
            !board_text.contains("DESK CHECK"),
            "desk-check is the team config's stage, not this workflow's; got:\n{board_text}"
        );

        let id = create_target_story(
            &pm,
            "[Feature] three-stage-walk-target",
            "reject if a three-stage configured middle cannot carry a story end-to-end",
        )
        .await;

        pm.call_tool(
            "move_story",
            serde_json::json!({"id": id, "target_state": "ready", "rationale": "schedule the falsifier walk"}),
        )
        .await
        .expect("pm schedules the story to ready");

        dev.call_tool(
            "start_story",
            serde_json::json!({
                "id": id,
                "pickup_paraphrase": "drive one story through the custom middle",
                "falsifier_reframing": "reject if any configured stage cannot be judged"
            }),
        )
        .await
        .expect("dev picks up into the first configured stage");

        let refused = designer.call_tool("pass_stage", pass_args(id)).await;
        let err = refused.expect_err("designer is not build's judge");
        assert!(
            err.contains("judge:build"),
            "the refusal names build's judgment: {err}"
        );

        dev.call_tool("pass_stage", pass_args(id))
            .await
            .expect("dev judges build");

        let story = pm
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            .expect("get_story mid-walk");
        assert_eq!(
            story.get("workflow_state").and_then(|v| v.as_str()),
            Some("review"),
            "a story at a configured stage reports THAT stage, not a collapsed 'in progress'; full={story}"
        );

        let refused = dev.call_tool("pass_stage", pass_args(id)).await;
        let err = refused.expect_err("dev is not review's judge — the story sits at review");
        assert!(
            err.contains("judge:review"),
            "the refusal names review's judgment: {err}"
        );

        designer
            .call_tool(
                "fail_stage",
                serde_json::json!({"id": id, "reason": "the falsifier scenario reproduces at review"}),
            )
            .await
            .expect("designer fails review");

        dev.call_tool("pass_stage", pass_args(id))
            .await
            .expect("the failed judgment landed the story back at build — dev may judge again");

        designer
            .call_tool("pass_stage", pass_args(id))
            .await
            .expect("designer judges review on the second pass");

        pm.call_tool("pass_stage", pass_args(id))
            .await
            .expect("harden configures NO judges — anyone may judge");

        pm.call_tool(
            "move_story",
            serde_json::json!({"id": id, "target_state": "accepted", "rationale": "falsifier walk complete"}),
        )
        .await
        .expect("pm concludes the story");

        tokio::time::sleep(Duration::from_millis(300)).await;
        let story = pm
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            .expect("get_story after the walk");
        assert_eq!(
            story.get("workflow_state").and_then(|v| v.as_str()),
            Some("accepted"),
            "the walk ends concluded; full={story}"
        );

        pm.close().await;
        dev.close().await;
        designer.close().await;
    });
}

#[test]
fn without_a_workflow_file_the_board_is_the_default_three_states() {
    let root = tempfile::TempDir::new().expect("bare project root");
    let bpmux = TestBpmux::start_with_envs(
        "human",
        &[
            ("BPMUX_TEST_MODE", "1"),
            ("BPMUX_PROJECT_ROOT", root.path().to_str().unwrap()),
        ],
    );
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let pm = TestBridge::connect(&bpmux, "pm").await;
        let dev = TestBridge::connect(&bpmux, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        ensure_current_iteration(&pm).await;

        let board = pm
            .call_tool("view_board", serde_json::json!({}))
            .await
            .expect("view_board renders");
        let board_text = board.as_str().unwrap_or_default().to_string();
        for header in ["ICEBOX", "READY", "IN PROGRESS", "ACCEPTANCE", "ACCEPTED"] {
            assert!(
                board_text.contains(header),
                "the default board carries {header}; got:\n{board_text}"
            );
        }
        assert!(
            !board_text.contains("DESK CHECK"),
            "no config file means ONE undivided middle — desk-check is configured, not default; got:\n{board_text}"
        );

        let id = create_target_story(
            &pm,
            "[Feature] default-workflow-walk-target",
            "reject if the no-config default cannot carry a story ready → in-progress → acceptance",
        )
        .await;
        pm.call_tool(
            "move_story",
            serde_json::json!({"id": id, "target_state": "ready", "rationale": "schedule the default walk"}),
        )
        .await
        .expect("pm schedules");
        dev.call_tool(
            "start_story",
            serde_json::json!({
                "id": id,
                "pickup_paraphrase": "walk the default three states",
                "falsifier_reframing": "reject if the single middle cannot pass judgment"
            }),
        )
        .await
        .expect("dev picks up");

        pm.call_tool("pass_stage", pass_args(id))
            .await
            .expect("the default middle configures no judges — anyone may judge");

        tokio::time::sleep(Duration::from_millis(300)).await;
        let story = pm
            .call_tool("get_story", serde_json::json!({"id": id}))
            .await
            .expect("get_story after the single judgment");
        assert_eq!(
            story.get("workflow_state").and_then(|v| v.as_str()),
            Some("acceptance"),
            "one judgment crosses the single middle into acceptance; full={story}"
        );

        pm.close().await;
        dev.close().await;
    });
}
