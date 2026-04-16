use super::tools_stories::gh;

pub trait StoryTracker: Clone + Send + Sync + 'static {
    fn close_story(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn add_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn remove_label(
        &self,
        number: u64,
        label: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn create_story(
        &self,
        title: &str,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn list_stories(&self) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn comment_on_story(
        &self,
        number: u64,
        body: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;
    fn get_story_title(
        &self,
        number: u64,
    ) -> impl std::future::Future<Output = Option<String>> + Send;
}

#[derive(Clone)]
pub struct GhTracker {
    pub board_fixture: Option<String>,
}

impl StoryTracker for GhTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        gh(&["issue", "close", &number.to_string()]).await
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        gh(&["issue", "edit", &number.to_string(), "--add-label", label]).await
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        gh(&[
            "issue",
            "edit",
            &number.to_string(),
            "--remove-label",
            label,
        ])
        .await
    }

    async fn create_story(&self, title: &str, body: &str) -> Result<String, String> {
        let mut args = vec!["issue", "create", "--title", title];
        if !body.is_empty() {
            args.extend(["--body", body]);
        }
        gh(&args).await
    }

    async fn list_stories(&self) -> Result<String, String> {
        if let Some(ref path) = self.board_fixture {
            return std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"));
        }
        gh(&[
            "issue",
            "list",
            "--state",
            "all",
            "--limit",
            "200",
            "--json",
            "number,title,state,labels",
        ])
        .await
    }

    async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        gh(&["issue", "comment", &number.to_string(), "--body", body]).await
    }

    async fn get_story_title(&self, number: u64) -> Option<String> {
        gh(&["issue", "view", &number.to_string(), "--json", "title"])
            .await
            .ok()
            .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
            .and_then(|v| v["title"].as_str().map(String::from))
    }
}

#[derive(Clone, Default)]
pub struct FakeTracker {
    pub board_fixture: Option<String>,
}

impl StoryTracker for FakeTracker {
    async fn close_story(&self, number: u64) -> Result<String, String> {
        Ok(format!("(fake) #{number} closed"))
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} labeled '{label}'"))
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} unlabeled '{label}'"))
    }

    async fn create_story(&self, title: &str, _body: &str) -> Result<String, String> {
        Ok(format!("(fake) created: {title}"))
    }

    async fn list_stories(&self) -> Result<String, String> {
        if let Some(ref path) = self.board_fixture {
            return std::fs::read_to_string(path).map_err(|e| format!("read fixture {path}: {e}"));
        }
        Ok("[]".to_string())
    }

    async fn comment_on_story(&self, number: u64, _body: &str) -> Result<String, String> {
        Ok(format!("(fake) #{number} commented"))
    }

    async fn get_story_title(&self, number: u64) -> Option<String> {
        Some(format!("(fake) story #{number}"))
    }
}
