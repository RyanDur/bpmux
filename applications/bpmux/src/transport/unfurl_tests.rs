#![allow(clippy::unwrap_used)]

use super::unfurl::extract_from_html;
use super::unfurl::*;
use std::time::Duration;

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
