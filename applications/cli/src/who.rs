pub fn run_who() -> anyhow::Result<()> {
    let (status, body) = crate::http::authed_get("/api/roster")?;
    if !status.is_success() {
        anyhow::bail!("roster fetch failed ({status}): {body}");
    }
    for line in who_lines(&body) {
        println!("{line}");
    }
    Ok(())
}

pub fn who_lines(body: &str) -> Vec<String> {
    let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or_default();
    let members = parsed["roster"].as_array().cloned().unwrap_or_default();
    if members.is_empty() {
        return vec!["no linked agents".to_string()];
    }
    members
        .iter()
        .map(|m| {
            format!(
                "{}  {}  {}  {}",
                m["id"].as_str().unwrap_or("?"),
                m["role"].as_str().unwrap_or("?"),
                m["identity"].as_str().unwrap_or("?"),
                m["presence"].as_str().unwrap_or("?"),
            )
        })
        .collect()
}
