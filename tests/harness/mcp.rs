//! Direct MCP client over HTTP to bpmux's own `/mcp` endpoint. Used by tests
//! that exercise bpmux's native MCP server (not the stdio bridge).

use std::time::Duration;

use rmcp::ServiceExt;
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;

use super::pty::TestBpmux;

/// MCP client connected over HTTP to bpmux's streamable-HTTP endpoint.
pub struct McpHttpClient {
    client: RunningService<rmcp::RoleClient, ()>,
}

impl McpHttpClient {
    /// Wait for the HTTP endpoint to be reachable, then complete the MCP
    /// handshake.
    pub async fn connect(bpmux: &TestBpmux) -> Self {
        let url = format!("http://127.0.0.1:{}/mcp", bpmux.port());
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if reqwest::get(&url).await.is_ok() {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("MCP HTTP endpoint should become reachable");
        let transport = StreamableHttpClientTransport::from_config(
            StreamableHttpClientTransportConfig::with_uri(url),
        );
        let client = ().serve(transport).await.expect("MCP handshake");
        Self { client }
    }

    pub async fn tool_names(&self) -> Vec<String> {
        self.client
            .peer()
            .list_all_tools()
            .await
            .expect("list tools")
            .iter()
            .map(|t| t.name.as_ref().to_string())
            .collect()
    }

    pub async fn send(&self, text: &str) {
        let mut params = rmcp::model::CallToolRequestParams::new("send_message");
        params.arguments = serde_json::json!({"text": text}).as_object().cloned();
        let result = self
            .client
            .peer()
            .call_tool(params)
            .await
            .expect("send_message");
        assert!(
            !result.is_error.unwrap_or(false),
            "send_message should succeed"
        );
    }

    /// Call `read_messages` and return the debug-formatted result text.
    /// Tests assert on substrings the way they would in a real agent.
    pub async fn read_all(&self) -> String {
        let params = rmcp::model::CallToolRequestParams::new("read_messages");
        let result = self
            .client
            .peer()
            .call_tool(params)
            .await
            .expect("read_messages");
        format!("{result:?}")
    }

    pub async fn set_name(&self, name: &str) -> String {
        let mut params = rmcp::model::CallToolRequestParams::new("set_name");
        params.arguments = serde_json::json!({"name": name}).as_object().cloned();
        let result = self
            .client
            .peer()
            .call_tool(params)
            .await
            .expect("set_name");
        assert!(!result.is_error.unwrap_or(false), "set_name should succeed");
        format!("{result:?}")
    }

    pub async fn set_status(&self, status: &str) -> String {
        let mut params = rmcp::model::CallToolRequestParams::new("set_status");
        params.arguments = serde_json::json!({"status": status}).as_object().cloned();
        let result = self
            .client
            .peer()
            .call_tool(params)
            .await
            .expect("set_status");
        assert!(
            !result.is_error.unwrap_or(false),
            "set_status should succeed"
        );
        format!("{result:?}")
    }

    pub async fn close(self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.client.cancel()).await;
    }
}
