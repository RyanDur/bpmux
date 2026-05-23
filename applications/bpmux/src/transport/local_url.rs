pub const DEFAULT_API_PORT: u16 = 4117;

const LOCAL_HOST: &str = "127.0.0.1";

pub fn api_url(port: u16, path: &str) -> String {
    format!("http://{LOCAL_HOST}:{port}{path}")
}

pub fn ws_url(port: u16, path: &str) -> String {
    format!("ws://{LOCAL_HOST}:{port}{path}")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn no_hardcoded_localhost_literals_in_src() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let src_dir = std::path::Path::new(manifest_dir).join("src");
        let mut offenders: Vec<String> = Vec::new();
        walk_rs_files(&src_dir, &mut |path: &std::path::Path| {
            let path_str = path.display().to_string();
            if path_str.ends_with("local_url.rs") {
                return;
            }
            let Ok(content) = std::fs::read_to_string(path) else {
                return;
            };
            for (lineno, line) in content.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if line.contains("http://127.0.0.1:") || line.contains("ws://127.0.0.1:") {
                    offenders.push(format!("{}:{}: {}", path_str, lineno + 1, line.trim()));
                }
            }
        });
        assert!(
            offenders.is_empty(),
            "#1364 Sc4 substrate-enforce: hardcoded localhost URL literals must route through transport::local_url helpers. Offenders:\n{}",
            offenders.join("\n")
        );
    }

    fn walk_rs_files(dir: &std::path::Path, callback: &mut dyn FnMut(&std::path::Path)) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk_rs_files(&path, callback);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                callback(&path);
            }
        }
    }

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
