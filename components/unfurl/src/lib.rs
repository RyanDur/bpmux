use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnfurlData {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub favicon: Option<String>,
}

#[derive(Default)]
pub struct UnfurlCache {
    entries: Mutex<HashMap<String, (UnfurlData, Instant)>>,
}

impl UnfurlCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, url: &str, ttl: Duration) -> Option<UnfurlData> {
        let guard = self.entries.lock().ok()?;
        let (data, at) = guard.get(url)?;
        if at.elapsed() > ttl {
            return None;
        }
        Some(data.clone())
    }

    pub fn put(&self, url: String, data: UnfurlData) {
        let Ok(mut guard) = self.entries.lock() else {
            return;
        };
        guard.insert(url, (data, Instant::now()));
    }
}

pub type UnfurlCacheHandle = Arc<UnfurlCache>;

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_BODY_BYTES: usize = 512 * 1024;

static META_RE: Lazy<Regex> = Lazy::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r#"(?i)<meta\s+(?:property|name)=["']([^"']+)["']\s+content=["']([^"']*)["']"#)
        .expect("meta-tag regex compiles")
});

static TITLE_RE: Lazy<Regex> = Lazy::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r"(?is)<title>(.+?)</title>").expect("title-tag regex compiles")
});

static FAVICON_RE: Lazy<Regex> = Lazy::new(|| {
    #[allow(clippy::expect_used)]
    Regex::new(r#"(?i)<link\s+rel=["'](?:shortcut\s+)?icon["']\s+href=["']([^"']+)["']"#)
        .expect("favicon regex compiles")
});

pub fn extract_from_html(url: &str, html: &str) -> UnfurlData {
    let mut data = UnfurlData {
        url: url.to_string(),
        ..Default::default()
    };

    for cap in META_RE.captures_iter(html) {
        let Some(key) = cap.get(1).map(|m| m.as_str().to_lowercase()) else {
            continue;
        };
        let Some(value) = cap.get(2).map(|m| m.as_str().to_string()) else {
            continue;
        };
        match key.as_str() {
            "og:title" => data.title = Some(value),
            "og:description" => data.description = Some(value),
            "og:image" => data.image = Some(value),
            "og:site_name" => data.site_name = Some(value),
            "twitter:title" if data.title.is_none() => data.title = Some(value),
            "twitter:description" if data.description.is_none() => data.description = Some(value),
            "twitter:image" if data.image.is_none() => data.image = Some(value),
            "description" if data.description.is_none() => data.description = Some(value),
            _ => {}
        }
    }

    if data.title.is_none()
        && let Some(cap) = TITLE_RE.captures(html)
        && let Some(m) = cap.get(1)
    {
        data.title = Some(m.as_str().trim().to_string());
    }

    if let Some(cap) = FAVICON_RE.captures(html)
        && let Some(m) = cap.get(1)
    {
        let raw = m.as_str();
        data.favicon = resolve_against_base(url, raw);
    }

    data
}

fn resolve_against_base(base: &str, href: &str) -> Option<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    let base_url = url::Url::parse(base).ok()?;
    base_url.join(href).ok().map(|u| u.to_string())
}

pub async fn fetch_html(url: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(FETCH_TIMEOUT)
        .user_agent("bpmux-unfurl/0.1")
        .build()
        .ok()?;
    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    let slice = &bytes[..bytes.len().min(MAX_BODY_BYTES)];
    Some(String::from_utf8_lossy(slice).into_owned())
}

#[cfg(test)]
mod extract_tests;
