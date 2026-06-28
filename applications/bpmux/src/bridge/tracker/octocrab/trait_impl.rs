use crate::bridge::tracker::{StoryTracker, WorkflowRunEntry};

use super::OctocrabStoryTracker;
use super::json_shape::issue_to_board_json;
use super::log_labels_fetch_failure;

impl StoryTracker for OctocrabStoryTracker {
    async fn close_story(&self, number: u64) -> Result<serde_json::Value, String> {
        self.set_issue_state(number, octocrab::models::IssueState::Closed)
            .await
    }

    async fn reopen_story(&self, number: u64) -> Result<serde_json::Value, String> {
        self.set_issue_state(number, octocrab::models::IssueState::Open)
            .await
    }

    async fn add_label(&self, number: u64, label: &str) -> Result<Vec<String>, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .add_labels(number, &[label.to_string()])
            .await
            .map(|labels| labels.into_iter().map(|l| l.name).collect())
            .map_err(Self::classify)
    }

    async fn remove_label(&self, number: u64, label: &str) -> Result<Vec<String>, String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, label)
            .await
        {
            Ok(labels) => Ok(labels.into_iter().map(|l| l.name).collect()),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(Vec::new())
            }
            Err(e) => Err(Self::classify(e)),
        }
    }

    async fn create_story(
        &self,
        title: &str,
        body: &str,
        initial_labels: &[&str],
    ) -> Result<serde_json::Value, String> {
        let issues = self.client.issues(&self.owner, &self.repo);
        let mut builder = issues.create(title);
        if !body.is_empty() {
            builder = builder.body(body);
        }
        if !initial_labels.is_empty() {
            builder = builder.labels(
                initial_labels
                    .iter()
                    .map(|l| l.to_string())
                    .collect::<Vec<_>>(),
            );
        }
        builder
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn update_body(&self, number: u64, body: &str) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .body(body)
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn update_title(&self, number: u64, title: &str) -> Result<serde_json::Value, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .title(title)
            .send()
            .await
            .map(|issue| issue_to_board_json(&issue))
            .map_err(Self::classify)
    }

    async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
        self.fetch_board_stories_since(None).await
    }

    async fn get_story(&self, number: u64) -> Result<Option<serde_json::Value>, String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
        {
            Ok(issue) => Ok(Some(issue_to_board_json(&issue))),
            Err(octocrab::Error::GitHub { source, .. }) if source.status_code.as_u16() == 404 => {
                Ok(None)
            }
            Err(e) => Err(Self::classify(e)),
        }
    }

    async fn comment_on_story(&self, number: u64, body: &str) -> Result<String, String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| format!("#{number} commented"))
            .map_err(Self::classify)
    }

    async fn list_story_comments(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
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

    async fn get_story_labels(&self, number: u64) -> Vec<String> {
        match self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
        {
            Ok(i) => i.labels.iter().map(|l| l.name.clone()).collect(),
            Err(e) => log_labels_fetch_failure("octocrab", number, Self::classify(e)),
        }
    }

    async fn list_latest_run_on_main(&self) -> Result<Option<WorkflowRunEntry>, String> {
        #[derive(serde::Deserialize)]
        struct RunsResponse {
            workflow_runs: Vec<WorkflowRunEntry>,
        }
        let path = format!(
            "/repos/{}/{}/actions/runs?branch=main&per_page=1",
            self.owner, self.repo
        );
        let response: RunsResponse = self.get_with_capture(path).await?;
        let Some(mut latest) = response.workflow_runs.into_iter().next() else {
            return Ok(None);
        };
        if latest.conclusion.as_deref() == Some("failure")
            && let Some(run_id) = latest.database_id
        {
            latest.annotation_message = self.fetch_first_failure_annotation(run_id).await;
        }
        Ok(Some(latest))
    }

    async fn list_sub_issues(&self, parent: u64) -> Result<Vec<u64>, String> {
        self.sub_issue_numbers(parent).await
    }

    async fn add_sub_issue(&self, parent: u64, child: u64) -> Result<(), String> {
        self.link_sub_issue(parent, child).await
    }

    async fn remove_sub_issue(&self, parent: u64, child: u64) -> Result<(), String> {
        self.unlink_sub_issue(parent, child).await
    }

    async fn reprioritize_sub_issue(
        &self,
        parent: u64,
        child: u64,
        after: Option<u64>,
    ) -> Result<(), String> {
        self.reorder_sub_issue(parent, child, after).await
    }
}
