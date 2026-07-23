use super::bridge::TestBridge;

pub fn extract_story_id(create_msg: &str) -> u64 {
    let tail = &create_msg[create_msg.find('#').expect("create message names #N") + 1..];
    let end = tail
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(tail.len());
    tail[..end].parse().expect("parse story number")
}

pub async fn ensure_current_iteration(pm: &TestBridge) -> u64 {
    let created = pm
        .call_tool(
            "create_iteration",
            serde_json::json!({"name": "Scheduling"}),
        )
        .await;
    let iteration_id = match created {
        Ok(msg) => extract_story_id(msg.as_str().unwrap_or_default()),
        Err(err) if err.contains("already open as #") => extract_story_id(&err),
        Err(err) => panic!("create scheduling iteration failed: {err}"),
    };
    pm.call_tool(
        "designate_current_iteration",
        serde_json::json!({"iteration_id": iteration_id}),
    )
    .await
    .expect("designate the scheduling iteration current");
    iteration_id
}

/// Drive a story through pickup + deliver so it lands in the
/// Delivered state, satisfying the #1385 slice-4 target-state-check
/// (accept requires Delivered). Caller passes a separate dev bridge
/// so the deliver-fire actor is different from the accept-fire actor
/// (sister-axis: slice-4 self-mutation guard refuses same-actor
/// deliver+accept).
pub async fn drive_to_delivered(
    pm: &TestBridge,
    dev: &TestBridge,
    story_id: u64,
    acceptance_test_path: &str,
) {
    ensure_current_iteration(pm).await;
    pm.call_tool(
        "move_story",
        serde_json::json!({
            "id": story_id,
            "target_state": "ready",
            "rationale": "drive_to_delivered: promote icebox→ready for downstream pickup"
        }),
    )
    .await
    .expect("move to ready");
    dev.call_tool(
        "start_story",
        serde_json::json!({
            "id": story_id,
            "pickup_paraphrase": "drive_to_delivered: pickup for state-check coverage",
            "falsifier_reframing": "drive_to_delivered: same falsifier as filing"
        }),
    )
    .await
    .expect("start_story");
    dev.call_tool(
        "deliver_story",
        serde_json::json!({
            "id": story_id,
            "verified_observation": format!(
                "drive_to_delivered: simulated deliver against {acceptance_test_path}"
            )
        }),
    )
    .await
    .expect("deliver_story");
}

pub async fn seed_team_verifiable_evidence(bpmux: &super::pty::TestBpmux, story_id: u64) {
    let pm = TestBridge::connect(bpmux, "pm").await;
    let designer = TestBridge::connect(bpmux, "designer").await;
    let dev = TestBridge::connect(bpmux, "dev").await;
    pm.call_tool(
        "label_story",
        serde_json::json!({
            "id": story_id,
            "label": "awaiting team",
            "rationale": "test-harness seed: acceptor-axis = team for team-verifiable acceptance"
        }),
    )
    .await
    .expect("pm label_story awaiting team");
    designer
        .call_tool(
            "submit_verdict",
            serde_json::json!({
                "story_id": story_id,
                "verdict": "verified",
                "substrate_grounding": "test-harness seed: designer verdict for accept-evidence"
            }),
        )
        .await
        .expect("designer submit_verdict");
    dev.call_tool(
        "submit_verdict",
        serde_json::json!({
            "story_id": story_id,
            "verdict": "verified",
            "substrate_grounding": "test-harness seed: dev verdict for accept-evidence"
        }),
    )
    .await
    .expect("dev submit_verdict");
    pm.close().await;
    designer.close().await;
    dev.close().await;
}

pub async fn seed_team_verifiable_evidence_with_mock_gh(
    bpmux: &super::pty::TestBpmux,
    mock_gh_base_url: &str,
    story_id: u64,
) {
    let pm = TestBridge::connect_with_mock_gh(bpmux, "pm", mock_gh_base_url).await;
    let designer = TestBridge::connect_with_mock_gh(bpmux, "designer", mock_gh_base_url).await;
    let dev = TestBridge::connect_with_mock_gh(bpmux, "dev", mock_gh_base_url).await;
    pm.call_tool(
        "label_story",
        serde_json::json!({
            "id": story_id,
            "label": "awaiting team",
            "rationale": "test-harness seed: acceptor-axis = team for team-verifiable acceptance"
        }),
    )
    .await
    .expect("pm label_story awaiting team");
    pm.close().await;
    designer
        .call_tool(
            "submit_verdict",
            serde_json::json!({
                "story_id": story_id,
                "verdict": "verified",
                "substrate_grounding": "test-harness seed: designer verdict for accept-evidence"
            }),
        )
        .await
        .expect("designer submit_verdict");
    dev.call_tool(
        "submit_verdict",
        serde_json::json!({
            "story_id": story_id,
            "verdict": "verified",
            "substrate_grounding": "test-harness seed: dev verdict for accept-evidence"
        }),
    )
    .await
    .expect("dev submit_verdict");
    designer.close().await;
    dev.close().await;
}

pub async fn create_target_story(pm: &TestBridge, title: &str, falsifier: &str) -> u64 {
    let atp = format!(
        "{}/tests/harness/story_helpers.rs",
        env!("CARGO_MANIFEST_DIR")
    );
    let create = pm
        .call_tool(
            "create_story",
            serde_json::json!({
                "title": title,
                "user_job": "When testing an affordance end-to-end, a target story must exist.",
                "observation_falsifier": falsifier,
                "acceptance_test_path": atp
            }),
        )
        .await
        .expect("create_story should succeed");
    let create_msg = create.as_str().unwrap_or("").to_string();
    extract_story_id(&create_msg)
}

pub fn structured_error(raw: &str) -> serde_json::Value {
    let text = raw.trim();
    let start = text.find('{').unwrap_or(0);
    let outer: serde_json::Value = serde_json::from_str(&text[start..])
        .unwrap_or_else(|e| panic!("refusal is not JSON ({e}): {raw}"));
    match &outer {
        serde_json::Value::Object(map) if map.contains_key("error") => {
            let inner = map["error"].as_str().unwrap_or_default();
            serde_json::from_str(inner).unwrap_or(outer)
        }
        _ => outer,
    }
}
