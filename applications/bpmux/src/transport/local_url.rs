pub const DEFAULT_API_PORT: u16 = 4117;

const LOCAL_HOST: &str = "127.0.0.1";

pub fn api_url(port: u16, path: &str) -> String {
    format!("http://{LOCAL_HOST}:{port}{path}")
}

pub fn ws_url(port: u16, path: &str) -> String {
    format!("ws://{LOCAL_HOST}:{port}{path}")
}

pub(crate) fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                String::from(b as char)
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}
