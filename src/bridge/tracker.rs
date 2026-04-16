use super::tools_stories::gh;

#[derive(Clone)]
pub enum StoryTracker {
    Gh,
    Fake,
}

impl StoryTracker {
    pub async fn close_story(&self, number: u64) -> Result<String, String> {
        match self {
            Self::Gh => gh(&["issue", "close", &number.to_string()]).await,
            Self::Fake => Ok(format!("(fake) #{number} closed")),
        }
    }

    pub async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        match self {
            Self::Gh => gh(&["issue", "edit", &number.to_string(), "--add-label", label]).await,
            Self::Fake => Ok(format!("(fake) #{number} labeled '{label}'")),
        }
    }

    pub async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        match self {
            Self::Gh => {
                gh(&[
                    "issue",
                    "edit",
                    &number.to_string(),
                    "--remove-label",
                    label,
                ])
                .await
            }
            Self::Fake => Ok(format!("(fake) #{number} unlabeled '{label}'")),
        }
    }

    pub async fn create_story(&self, title: &str, body: &str) -> Result<String, String> {
        match self {
            Self::Gh => {
                let mut args = vec!["issue", "create", "--title", title];
                if !body.is_empty() {
                    args.extend(["--body", body]);
                }
                gh(&args).await
            }
            Self::Fake => Ok(format!("(fake) created: {title}")),
        }
    }

    pub async fn list_stories(&self) -> Result<String, String> {
        if let Ok(path) = std::env::var("BPMUX_BOARD_FIXTURE") {
            return std::fs::read_to_string(&path).map_err(|e| format!("read fixture {path}: {e}"));
        }
        match self {
            Self::Gh => {
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
            Self::Fake => Ok("[]".to_string()),
        }
    }

    pub async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        match self {
            Self::Gh => gh(&["issue", "comment", &number.to_string(), "--body", body]).await,
            Self::Fake => Ok(format!("(fake) #{number} commented")),
        }
    }

    pub async fn get_story_title(&self, number: u64) -> Option<String> {
        match self {
            Self::Gh => gh(&["issue", "view", &number.to_string(), "--json", "title"])
                .await
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|v| v["title"].as_str().map(String::from)),
            Self::Fake => Some(format!("(fake) story #{number}")),
        }
    }
}
