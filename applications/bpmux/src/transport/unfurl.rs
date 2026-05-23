use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::rest::json_headers;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UnfurlData {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub favicon: Option<String>,
}

#[derive(Deserialize)]
pub struct UnfurlQuery {
    pub url: String,
}

#[derive(Default)]
pub struct UnfurlCache {
    entries: Mutex<HashMap<String, (UnfurlData, Instant)>>,
}

impl UnfurlCache {
    pub fn new() -> Self {
        Self::default()
    }

    fn get(&self, url: &str, ttl: Duration) -> Option<UnfurlData> {
        let guard = self.entries.lock().ok()?;
        let (data, at) = guard.get(url)?;
        if at.elapsed() > ttl {
            return None;
        }
        Some(data.clone())
    }

    fn put(&self, url: String, data: UnfurlData) {
        let Ok(mut guard) = self.entries.lock() else {
            return;
        };
        guard.insert(url, (data, Instant::now()));
    }
}

pub type UnfurlCacheHandle = Arc<UnfurlCache>;

const CACHE_TTL: Duration = Duration::from_secs(3600);
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

fn extract_from_html(url: &str, html: &str) -> UnfurlData {
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

async fn fetch_html(url: &str) -> Option<String> {
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

pub async fn get_unfurl(
    State(cache): State<UnfurlCacheHandle>,
    Query(params): Query<UnfurlQuery>,
) -> impl IntoResponse {
    let Ok(parsed) = url::Url::parse(&params.url) else {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            r#"{"error":"invalid url"}"#.to_string(),
        );
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        return (
            StatusCode::BAD_REQUEST,
            json_headers(),
            r#"{"error":"unsupported scheme"}"#.to_string(),
        );
    }

    if let Some(cached) = cache.get(&params.url, CACHE_TTL) {
        return (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&cached).unwrap_or_else(|_| "{}".to_string()),
        );
    }

    let Some(html) = fetch_html(&params.url).await else {
        let data = UnfurlData {
            url: params.url.clone(),
            ..Default::default()
        };
        cache.put(params.url.clone(), data.clone());
        return (
            StatusCode::OK,
            json_headers(),
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
        );
    };

    let data = extract_from_html(&params.url, &html);
    cache.put(params.url.clone(), data.clone());
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn extracts_og_tags() {
        let html = r#"
            <html><head>
                <meta property="og:title" content="My Page">
                <meta property="og:description" content="Some description">
                <meta property="og:image" content="https://example.com/img.png">
            </head></html>
        "#;
        let data = extract_from_html("https://example.com", html);
        assert_eq!(data.title.as_deref(), Some("My Page"));
        assert_eq!(data.description.as_deref(), Some("Some description"));
        assert_eq!(data.image.as_deref(), Some("https://example.com/img.png"));
    }

    #[test]
    fn falls_back_to_title_tag_when_og_missing() {
        let html = "<html><head><title>Fallback Title</title></head></html>";
        let data = extract_from_html("https://example.com", html);
        assert_eq!(data.title.as_deref(), Some("Fallback Title"));
    }

    #[test]
    fn resolves_relative_favicon() {
        let html = r#"<html><head><link rel="icon" href="/favicon.ico"></head></html>"#;
        let data = extract_from_html("https://example.com/path", html);
        assert_eq!(
            data.favicon.as_deref(),
            Some("https://example.com/favicon.ico")
        );
    }

    #[test]
    fn twitter_card_fills_gaps_when_og_missing() {
        let html = r#"
            <meta name="twitter:title" content="TW Title">
            <meta name="twitter:description" content="TW Description">
        "#;
        let data = extract_from_html("https://example.com", html);
        assert_eq!(data.title.as_deref(), Some("TW Title"));
        assert_eq!(data.description.as_deref(), Some("TW Description"));
    }

    #[test]
    fn cache_returns_none_when_ttl_expired() {
        let cache = UnfurlCache::new();
        cache.put(
            "https://example.com".to_string(),
            UnfurlData {
                url: "https://example.com".to_string(),
                title: Some("cached".to_string()),
                ..Default::default()
            },
        );
        assert!(
            cache
                .get("https://example.com", Duration::from_secs(100))
                .is_some()
        );
        assert!(
            cache
                .get("https://example.com", Duration::from_nanos(1))
                .is_none()
        );
    }
}
