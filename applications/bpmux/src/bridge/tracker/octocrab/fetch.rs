use super::super::trait_def::ConditionalIssue;
use super::OctocrabStoryTracker;
use super::json_shape::issue_to_board_json;

impl OctocrabStoryTracker {
    pub async fn get_story_conditional(
        &self,
        number: u64,
        prior_etag: Option<&str>,
    ) -> Result<ConditionalIssue, String> {
        use http_body_util::BodyExt;
        let path = format!("/repos/{}/{}/issues/{number}", self.owner, self.repo);
        let mut headers = http::HeaderMap::new();
        if let Some(etag) = prior_etag
            && let Ok(value) = http::HeaderValue::from_str(etag)
        {
            headers.insert("if-none-match", value);
        }
        let response = self
            .client
            ._get_with_headers(path, Some(headers))
            .await
            .map_err(Self::classify)?;
        crate::rate_limit::capture_headers(
            response.headers(),
            &self.rate_limit_observer,
            std::time::SystemTime::now(),
        );
        let status = response.status();
        let etag = response
            .headers()
            .get("etag")
            .and_then(|v| v.to_str().ok())
            .map(ToString::to_string);
        if status.as_u16() == 304 {
            return Ok(ConditionalIssue::NotModified);
        }
        if status.as_u16() == 404 {
            return Ok(ConditionalIssue::NotFound);
        }
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("body collect: {e}"))?
            .to_bytes();
        if !status.is_success() {
            return Err(format!(
                "github error {status}: {}",
                String::from_utf8_lossy(&body)
            ));
        }
        let issue: octocrab::models::issues::Issue =
            serde_json::from_slice(&body).map_err(|e| format!("json parse: {e}"))?;
        Ok(ConditionalIssue::Modified {
            value: issue_to_board_json(&issue),
            etag,
        })
    }

    pub async fn fetch_board_stories_since(
        &self,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let since_q = since.map(|s| {
            format!(
                "&since={}",
                s.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            )
        });
        let mut all_issues: Vec<octocrab::models::issues::Issue> = Vec::new();
        let mut page = 1u32;
        loop {
            let path = format!(
                "/repos/{}/{}/issues?state=open&per_page=100&page={}{}",
                self.owner,
                self.repo,
                page,
                since_q.as_deref().unwrap_or("")
            );
            let issues: Vec<octocrab::models::issues::Issue> = self.get_with_capture(path).await?;
            let count = issues.len();
            all_issues.extend(issues);
            if count < 100 {
                break;
            }
            page += 1;
        }
        let open_count = all_issues.len();
        let mut closed_count = 0usize;
        let mut closed_page = 1u32;
        loop {
            let closed_path = format!(
                "/repos/{}/{}/issues?state=closed&per_page=100&sort=updated&direction=desc&page={}{}",
                self.owner,
                self.repo,
                closed_page,
                since_q.as_deref().unwrap_or("")
            );
            let closed: Vec<octocrab::models::issues::Issue> =
                self.get_with_capture(closed_path).await?;
            let count = closed.len();
            all_issues.extend(closed);
            closed_count += count;
            if count < 100 {
                break;
            }
            closed_page += 1;
        }
        crate::diag::log_diag(&format!(
            "github fetch_board_stories_since(since={:?}): {open_count} open (paginated) + {closed_count} closed (paginated) = {} total",
            since.map(|s| s.to_rfc3339()),
            all_issues.len()
        ));
        Ok(all_issues.iter().map(issue_to_board_json).collect())
    }
}
