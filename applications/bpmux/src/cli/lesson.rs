use super::http::http_get;

pub(crate) fn run_lesson_save(
    topic: &str,
    lesson: &str,
    evidence: &[String],
) -> anyhow::Result<()> {
    let role = crate::resolve_role(&None);
    let port = crate::resolve_bpmux_port();
    let body = serde_json::json!({
        "topic": topic,
        "lesson": lesson,
        "author_role": role,
        "evidence": evidence,
    });
    let url = crate::transport::local_url::api_url(port, "/api/lessons");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let (status, text) = rt.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(5_000))
            .build()?;
        let resp = client.post(&url).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Ok::<_, reqwest::Error>((status, text))
    })?;
    if !status.is_success() {
        anyhow::bail!("bpmux lesson save rejected ({status}): {text}");
    }
    println!("lesson saved under '{topic}'");
    Ok(())
}

pub(crate) fn run_lesson_list(topic: &Option<String>, limit: Option<u64>) -> anyhow::Result<()> {
    let port = crate::resolve_bpmux_port();
    let mut url = crate::transport::local_url::api_url(port, "/api/lessons");
    let mut has_param = false;
    if let Some(t) = topic {
        url.push_str(&format!("?topic={t}"));
        has_param = true;
    }
    if let Some(n) = limit {
        url.push_str(if has_param { "&" } else { "?" });
        url.push_str(&format!("limit={n}"));
    }
    let (status, body) = http_get(&url)?;
    if !status.is_success() {
        anyhow::bail!("lessons fetch failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}

pub(crate) fn run_lesson_list_peer(peer: &Option<String>) -> anyhow::Result<()> {
    let port = crate::resolve_bpmux_port();
    let mut url = crate::transport::local_url::api_url(port, "/api/peer-lessons");
    if let Some(p) = peer {
        url.push_str(&format!("?peer={p}"));
    }
    let (status, body) = http_get(&url)?;
    if !status.is_success() {
        anyhow::bail!("peer-lessons fetch failed ({status}): {body}");
    }
    println!("{body}");
    Ok(())
}
