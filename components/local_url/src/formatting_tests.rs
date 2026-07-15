use crate::*;

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

#[test]
fn percent_encode_passes_unreserved_and_escapes_the_rest() {
    assert_eq!(percent_encode("abcXYZ090-_.~"), "abcXYZ090-_.~");
    assert_eq!(percent_encode("a b&c=d"), "a%20b%26c%3Dd");
}
