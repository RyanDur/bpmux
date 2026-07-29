#![allow(dead_code)]

use super::OctocrabStoryTracker;

const SUB_ISSUES_API_VERSION: &str = "2026-03-10";

#[derive(serde::Deserialize)]
struct IssueIdentity {
    id: u64,
    number: u64,
}

impl OctocrabStoryTracker {
    pub(super) async fn sub_issue_numbers(&self, parent: u64) -> Result<Vec<u64>, String> {
        let children = self.fetch_sub_issue_identities(parent).await?;
        Ok(children.into_iter().map(|c| c.number).collect())
    }

    pub(super) async fn link_sub_issue(&self, parent: u64, child: u64) -> Result<(), String> {
        let child_id = self.resolve_issue_id(child).await?;
        let path = format!(
            "/repos/{}/{}/issues/{parent}/sub_issues",
            self.owner, self.repo
        );
        self.send_versioned(
            http::Method::POST,
            path,
            Some(serde_json::json!({ "sub_issue_id": child_id })),
        )
        .await
        .map(|_| ())
    }

    pub(super) async fn unlink_sub_issue(&self, parent: u64, child: u64) -> Result<(), String> {
        let child_id = self.resolve_issue_id(child).await?;
        let path = format!(
            "/repos/{}/{}/issues/{parent}/sub_issue",
            self.owner, self.repo
        );
        self.send_versioned(
            http::Method::DELETE,
            path,
            Some(serde_json::json!({ "sub_issue_id": child_id })),
        )
        .await
        .map(|_| ())
    }

    pub(super) async fn reorder_sub_issue(
        &self,
        parent: u64,
        child: u64,
        after: Option<u64>,
    ) -> Result<(), String> {
        let child_id = self.resolve_issue_id(child).await?;
        let mut body = serde_json::Map::new();
        body.insert("sub_issue_id".into(), child_id.into());
        match after {
            Some(after_number) => {
                let after_id = self.resolve_issue_id(after_number).await?;
                body.insert("after_id".into(), after_id.into());
            }
            None => match self.front_anchor_id(parent, child_id).await? {
                Some(front_id) => {
                    body.insert("before_id".into(), front_id.into());
                }
                None => return Ok(()),
            },
        }
        let path = format!(
            "/repos/{}/{}/issues/{parent}/sub_issues/priority",
            self.owner, self.repo
        );
        self.send_versioned(
            http::Method::PATCH,
            path,
            Some(serde_json::Value::Object(body)),
        )
        .await
        .map(|_| ())
    }

    async fn front_anchor_id(&self, parent: u64, child_id: u64) -> Result<Option<u64>, String> {
        let children = self.fetch_sub_issue_identities(parent).await?;
        Ok(children
            .into_iter()
            .map(|c| c.id)
            .find(|&id| id != child_id))
    }

    async fn resolve_issue_id(&self, number: u64) -> Result<u64, String> {
        let path = format!("/repos/{}/{}/issues/{number}", self.owner, self.repo);
        let identity: IssueIdentity = self.get_versioned(path).await?;
        Ok(identity.id)
    }

    async fn fetch_sub_issue_identities(&self, parent: u64) -> Result<Vec<IssueIdentity>, String> {
        let path = format!(
            "/repos/{}/{}/issues/{parent}/sub_issues",
            self.owner, self.repo
        );
        self.get_versioned(path).await
    }

    async fn get_versioned<R: serde::de::DeserializeOwned>(
        &self,
        path: String,
    ) -> Result<R, String> {
        let bytes = self.send_versioned(http::Method::GET, path, None).await?;
        serde_json::from_slice(&bytes).map_err(|e| format!("json parse: {e}"))
    }

    async fn send_versioned(
        &self,
        method: http::Method,
        path: String,
        body: Option<serde_json::Value>,
    ) -> Result<Vec<u8>, String> {
        use http_body_util::BodyExt;
        let builder = http::request::Builder::new()
            .method(method)
            .uri(path)
            .header("X-GitHub-Api-Version", SUB_ISSUES_API_VERSION);
        let request = self
            .client
            .build_request(builder, body.as_ref())
            .map_err(Self::classify)?;
        let response = self.client.execute(request).await.map_err(Self::classify)?;
        rate_limit::capture_headers(
            response.headers(),
            &self.rate_limit_observer,
            std::time::SystemTime::now(),
        );
        let status = response.status();
        let bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| format!("body collect: {e}"))?
            .to_bytes();
        if !status.is_success() {
            return Err(super::errors::classify_response(
                status.as_u16(),
                &String::from_utf8_lossy(&bytes),
            )
            .user_message());
        }
        Ok(bytes.to_vec())
    }
}
