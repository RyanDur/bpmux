//! Single-source-of-truth for the local bpmux backend's URL prefix
//! and default port (#1364).
//!
//! Pre-Andromeda, 18+ callsites across `lib.rs`, `bridge/rpc_client.rs`,
//! `bridge/tools/*` and sister modules formatted local URLs as
//! `format!("http://127.0.0.1:{port}/api/...")`. The duplication
//! made it impossible to grep for "the bpmux URL prefix" or change
//! the listen-host without touching every callsite. Sister to
//! Ryan-canon 2026-05-16 21:00 UTC "watch out for magic numbers
//! and strings."
//!
//! Use [`api_url`] / [`ws_url`] from any callsite that needs to
//! talk to the local bpmux backend. [`DEFAULT_API_PORT`] replaces
//! all hardcoded `4117` literals.

/// Default port the bpmux HTTP + WebSocket + MCP server binds to.
/// Honors `BPMUX_PORT` env override at callers that explicitly
/// query it (e.g., [`crate::resolve_bpmux_port`]).
pub const DEFAULT_API_PORT: u16 = 4117;

const LOCAL_HOST: &str = "127.0.0.1";

/// Build the bpmux HTTP API URL for a given port and request path.
/// Path is appended verbatim; callers control the leading slash.
pub fn api_url(port: u16, path: &str) -> String {
    format!("http://{LOCAL_HOST}:{port}{path}")
}

/// Build the bpmux WebSocket URL for a given port and request path.
/// Path is appended verbatim; callers control the leading slash.
pub fn ws_url(port: u16, path: &str) -> String {
    format!("ws://{LOCAL_HOST}:{port}{path}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn api_url_prefixes_http_and_local_host() {
        assert_eq!(
            api_url(4117, "/api/messages"),
            "http://127.0.0.1:4117/api/messages"
        );
    }

    #[test]
    fn ws_url_prefixes_ws_and_local_host() {
        assert_eq!(ws_url(4117, "/ws"), "ws://127.0.0.1:4117/ws");
    }

    #[test]
    fn api_url_passes_path_verbatim_with_query_string() {
        let path = "/api/messages?limit=200&chat_only=true";
        assert_eq!(
            api_url(8080, path),
            "http://127.0.0.1:8080/api/messages?limit=200&chat_only=true"
        );
    }

    #[test]
    fn default_api_port_pins_4117() {
        assert_eq!(DEFAULT_API_PORT, 4117);
    }
}
