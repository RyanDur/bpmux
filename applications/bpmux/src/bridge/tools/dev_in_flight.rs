use serde::Serialize;

use crate::bridge::tracker::StoryTracker;

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub(in crate::bridge) async fn handle_dev_in_flight(
    tracker: &impl StoryTracker,
) -> Result<String, String> {
    let stories = tracker.fetch_board_stories().await?;
    let in_flight = stories.iter().any(|s| {
        let labels = s.get("labels").and_then(|v| v.as_array());
        match labels {
            Some(arr) => arr.iter().any(|l| {
                l.as_str()
                    .map(|name| name == "in progress" || name == "started" || name == "finished")
                    .unwrap_or(false)
            }),
            None => false,
        }
    });
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct BoardFixtureTracker {
        labels_per_story: Vec<Vec<&'static str>>,
    }

    impl StoryTracker for BoardFixtureTracker {
        async fn close_story(&self, _n: u64) -> Result<String, String> {
            Ok("".into())
        }
        async fn add_label(&self, _n: u64, _l: &str) -> Result<String, String> {
            Ok("".into())
        }
        async fn remove_label(&self, _n: u64, _l: &str) -> Result<String, String> {
            Ok("".into())
        }
        async fn create_story(&self, _t: &str, _b: &str) -> Result<String, String> {
            Ok("".into())
        }
        async fn list_stories(&self) -> Result<String, String> {
            Ok("[]".into())
        }
        async fn fetch_board_stories(&self) -> Result<Vec<serde_json::Value>, String> {
            Ok(self
                .labels_per_story
                .iter()
                .enumerate()
                .map(|(i, labels)| {
                    serde_json::json!({
                        "number": i + 1,
                        "labels": labels,
                    })
                })
                .collect())
        }
        async fn get_story(&self, _n: u64) -> Result<Option<serde_json::Value>, String> {
            Ok(None)
        }
        async fn comment_on_story(&self, _n: u64, _b: &str) -> Result<String, String> {
            Ok("".into())
        }
        async fn list_story_comments(&self, _n: u64) -> Result<Vec<serde_json::Value>, String> {
            Ok(Vec::new())
        }
        async fn get_story_title(&self, _n: u64) -> Option<String> {
            None
        }
        async fn get_story_labels(&self, _n: u64) -> Vec<String> {
            Vec::new()
        }
        async fn list_latest_run_on_main(
            &self,
        ) -> Result<Option<crate::bridge::tracker::WorkflowRunEntry>, String> {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn returns_false_when_no_story_in_flight() {
        let tracker = BoardFixtureTracker {
            labels_per_story: vec![vec!["ready"], vec!["delivered"], vec!["accepted"]],
        };
        let result = handle_dev_in_flight(&tracker).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":false"));
    }

    #[tokio::test]
    async fn returns_true_when_a_story_is_in_progress() {
        let tracker = BoardFixtureTracker {
            labels_per_story: vec![vec!["ready"], vec!["in progress"], vec!["delivered"]],
        };
        let result = handle_dev_in_flight(&tracker).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":true"));
    }

    #[tokio::test]
    async fn returns_true_when_legacy_started_label_present() {
        let tracker = BoardFixtureTracker {
            labels_per_story: vec![vec!["started"]],
        };
        let result = handle_dev_in_flight(&tracker).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":true"));
    }

    #[tokio::test]
    async fn returns_true_when_finished_label_present() {
        let tracker = BoardFixtureTracker {
            labels_per_story: vec![vec!["finished"]],
        };
        let result = handle_dev_in_flight(&tracker).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":true"));
    }

    #[tokio::test]
    async fn response_contains_no_story_identifiers() {
        let tracker = BoardFixtureTracker {
            labels_per_story: vec![vec!["in progress"]],
        };
        let result = handle_dev_in_flight(&tracker).await.unwrap();
        assert!(!result.contains("number"));
        assert!(!result.contains("title"));
    }
}
