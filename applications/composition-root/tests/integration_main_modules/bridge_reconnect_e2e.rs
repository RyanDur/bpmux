#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::time::{Duration, Instant};

use crate::harness::bridge::TestBridge;
use crate::harness::port::free_port;
use crate::harness::pty::TestBpmux;

fn wait_for_port_close(port: u16, deadline: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < deadline {
        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{port}").parse().unwrap(),
            Duration::from_millis(100),
        )
        .is_err()
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn bridge_auto_reconnects_to_restarted_server_within_5s_on_same_port() {
    let rt = tokio::runtime::Runtime::new().expect("rt");
    rt.block_on(async {
        let port = free_port();
        let data_base_dir_a = tempfile::TempDir::new().expect("temp");
        let server_a = TestBpmux::start_with_session_and_port("dev", data_base_dir_a, port);
        tokio::time::sleep(Duration::from_millis(500)).await;
        let bridge = TestBridge::connect(&server_a, "dev").await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        bridge
            .call_tool("set_name", serde_json::json!({ "name": "dev" }))
            .await
            .expect("baseline set_name on server A must succeed");

        drop(server_a);
        assert!(
            wait_for_port_close(port, Duration::from_secs(5)),
            "server A's port {port} must release within 5s of drop"
        );

        let respawn_deadline = Instant::now() + Duration::from_secs(5);
        let mut server_b = None;
        let mut last_err: Option<String> = None;
        while Instant::now() < respawn_deadline {
            let data_base_dir_b = tempfile::TempDir::new().expect("temp");
            let result = std::panic::catch_unwind(|| {
                TestBpmux::start_with_session_and_port("dev", data_base_dir_b, port)
            });
            match result {
                Ok(b) => {
                    server_b = Some(b);
                    break;
                }
                Err(e) => {
                    last_err = Some(format!("{e:?}"));
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
        let _server_b = server_b
            .unwrap_or_else(|| panic!("server B never bound port {port} (last: {last_err:?})"));

        let reconnect_start = Instant::now();
        let mut last_err: Option<String> = None;
        loop {
            if reconnect_start.elapsed() > Duration::from_secs(5) {
                panic!(
                    "bridge did not auto-reconnect to server B within 5s; last err: {last_err:?}"
                );
            }
            match bridge
                .call_tool("set_name", serde_json::json!({ "name": "dev" }))
                .await
            {
                Ok(_) => break,
                Err(e) => {
                    last_err = Some(e.to_string());
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }
        let elapsed = reconnect_start.elapsed();
        assert!(
            elapsed < Duration::from_secs(5),
            "bridge auto-reconnect must complete within 5s for falsifier-(b); got {elapsed:?}"
        );
    });
}
