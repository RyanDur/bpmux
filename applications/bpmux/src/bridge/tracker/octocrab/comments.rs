use super::OctocrabStoryTracker;

impl OctocrabStoryTracker {
    pub(super) async fn create_story_comment(
        &self,
        number: u64,
        body: &str,
    ) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| format!("#{number} commented"))
            .map_err(Self::classify)
    }

    pub(super) async fn story_comments_condensed(
        &self,
        number: u64,
    ) -> Result<Vec<serde_json::Value>, String> {
        let page = self
            .client
            .issues(&self.owner, &self.repo)
            .list_comments(number)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        let mut condensed: Vec<serde_json::Value> = page
            .items
            .into_iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.into_inner(),
                    "author": c.user.login,
                    "body": c.body.unwrap_or_default(),
                    "created_at": c.created_at.to_rfc3339(),
                })
            })
            .collect();
        condensed.sort_by(|a, b| {
            let ak = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            let bk = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
            ak.cmp(bk)
        });
        Ok(condensed)
    }
}
