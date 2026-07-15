#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
fn no_hardcoded_localhost_literals_in_src() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let src_dir = std::path::Path::new(manifest_dir).join("src");
    let mut offenders: Vec<String> = Vec::new();
    walk_rs_files(&src_dir, &mut |path: &std::path::Path| {
        let path_str = path.display().to_string();
        if path_str.ends_with("local_url_lock_tests.rs") {
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
        "#1364 Sc4 substrate-enforce: hardcoded localhost URL literals must route through the local_url component helpers. Offenders:\n{}",
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
