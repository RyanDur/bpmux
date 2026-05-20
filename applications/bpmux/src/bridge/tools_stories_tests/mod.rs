#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod acceptance_test_path_tests;
mod add_story_to_iteration_tests;
mod atp_validator_tests;
mod handler_unit_tests;
mod integration_drain_tests;
mod label_story_iteration_retire_tests;
mod lifecycle_handler_tests;
mod mutation_handler_tests;
mod notification_bridge_tests;
mod observation_falsifier_tests;
mod pickup_tests;
mod reopen_transition_tests;
mod reorder_icebox_tests;
mod role_tests;
mod signal_tests;
mod stale_workflow_labels_tests;
mod verdict_substrate_tests;
mod view_board_basic_tests;
mod view_board_epic_tests;

pub(super) fn test_ci_state_store_green() -> crate::transport::ci_state_store::CiStateStore {
    let store = crate::transport::ci_state_store::CiStateStore::new();
    store.write(crate::bridge::tools::CiStateReport {
        conclusion: "success",
        last_run_id: None,
        branch: "main",
        url: String::new(),
        head_sha: Some("test-fixture-sha".to_string()),
        display_title: None,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 0,
        backoff_cap_secs: 0,
        workflows: Vec::new(),
    });
    store
}

pub(super) async fn connected_ws_sink_with_capture() -> (
    tokio::sync::Mutex<Option<crate::bridge::tools::WsSink>>,
    std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
) {
    use futures::StreamExt;
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured: std::sync::Arc<tokio::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let captured_for_task = std::sync::Arc::clone(&captured);
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await
            && let Ok(ws) = tokio_tungstenite::accept_async(stream).await
        {
            let (_srv_sink, mut srv_stream) = ws.split();
            while let Some(Ok(msg)) = srv_stream.next().await {
                if let Ok(text) = msg.into_text() {
                    captured_for_task.lock().await.push(text.to_string());
                }
            }
        }
    });
    let url = format!("ws://{}/", addr);
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (sink, stream) = ws.split();
    tokio::spawn(async move {
        let mut stream = stream;
        while stream.next().await.is_some() {}
    });
    (tokio::sync::Mutex::new(Some(sink)), captured)
}

pub(super) async fn connected_ws_sink_for_test()
-> tokio::sync::Mutex<Option<crate::bridge::tools::WsSink>> {
    use futures::StreamExt;
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await
            && let Ok(ws) = tokio_tungstenite::accept_async(stream).await
        {
            let (_srv_sink, mut srv_stream) = ws.split();
            while srv_stream.next().await.is_some() {}
        }
    });
    let url = format!("ws://{}/", addr);
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (sink, stream) = ws.split();
    tokio::spawn(async move {
        let mut stream = stream;
        while stream.next().await.is_some() {}
    });
    tokio::sync::Mutex::new(Some(sink))
}
