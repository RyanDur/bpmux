use super::OctocrabStoryTracker;

impl OctocrabStoryTracker {
    pub async fn probe(&self) -> Result<(), String> {
        let path = format!("/repos/{}/{}/issues?per_page=1", self.owner, self.repo);
        let response = self
            .client
            ._get_with_headers(path, None)
            .await
            .map_err(Self::classify)?;
        match response.status().as_u16() {
            200 => Ok(()),
            401 | 403 => Err("the token was refused".to_string()),
            404 => Err(format!(
                "repository {}/{} not found, or the token cannot see it",
                self.owner, self.repo
            )),
            status => Err(format!("github answered {status}")),
        }
    }
}
