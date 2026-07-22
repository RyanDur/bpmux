use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::time::Duration;

use super::rest::json_headers;

pub use ::unfurl::{UnfurlCache, UnfurlCacheHandle, UnfurlData};

#[derive(Deserialize)]
pub struct UnfurlQuery {
    pub url: String,
}

const CACHE_TTL: Duration = Duration::from_secs(3600);

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

    let Some(html) = ::unfurl::fetch_html(&params.url).await else {
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

    let data = ::unfurl::extract_from_html(&params.url, &html);
    cache.put(params.url.clone(), data.clone());
    (
        StatusCode::OK,
        json_headers(),
        serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
    )
}
