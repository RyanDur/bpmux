const SHORT_SHA_LEN: usize = 7;

pub fn short_sha(sha: &str) -> String {
    sha.chars().take(SHORT_SHA_LEN).collect()
}

pub fn build_precommit_activity_payload(role: &str, sha: &str) -> serde_json::Value {
    let short = short_sha(sha);
    let text = format!("pre-commit gates passed @ {short} (cargo fmt + clippy)");
    serde_json::json!({
        "from": format!("{role}:pre-commit-hook"),
        "text": text,
        "message_type": "activity",
    })
}
