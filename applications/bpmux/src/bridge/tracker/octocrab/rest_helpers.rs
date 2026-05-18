use super::OctocrabStoryTracker;

impl OctocrabStoryTracker {
    pub async fn accept_story(&self, number: u64) -> Result<(), String> {
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, "delivered")
            .await
        {
            match e {
                octocrab::Error::GitHub { ref source, .. }
                    if source.status_code.as_u16() == 404 => {}
                other => {
                    crate::session::diag::log_diag(&format!(
                        "rest: accept_story remove-label delivered failed for #{number}: {}",
                        Self::classify(other)
                    ));
                }
            }
        }
        self.client
            .issues(&self.owner, &self.repo)
            .update(number)
            .state(octocrab::models::IssueState::Closed)
            .send()
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }

    pub async fn reject_story(&self, number: u64, note: &str) -> Result<(), String> {
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .remove_label(number, "delivered")
            .await
        {
            match e {
                octocrab::Error::GitHub { ref source, .. }
                    if source.status_code.as_u16() == 404 => {}
                other => {
                    crate::session::diag::log_diag(&format!(
                        "rest: reject_story remove-label delivered failed for #{number}: {}",
                        Self::classify(other)
                    ));
                }
            }
        }
        if let Err(e) = self
            .client
            .issues(&self.owner, &self.repo)
            .add_labels(number, &["rejected".to_string()])
            .await
        {
            crate::session::diag::log_diag(&format!(
                "rest: reject_story add-label rejected failed for #{number}: {}",
                Self::classify(e)
            ));
        }
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, note)
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }

    pub async fn list_story_timeline(&self, number: u64) -> Result<Vec<serde_json::Value>, String> {
        let issue = self
            .client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .map_err(Self::classify)?;
        let mut out: Vec<serde_json::Value> = vec![serde_json::json!({
            "event": "filed",
            "actor": issue.user.login,
            "ts": issue.created_at.to_rfc3339(),
        })];

        let timeline = self
            .client
            .issues(&self.owner, &self.repo)
            .list_timeline_events(number)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        for e in timeline.items {
            let Some(ts) = e.created_at else { continue };
            let actor = e
                .actor
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_default();
            let event = serde_json::to_value(&e.event)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            let mut entry = serde_json::json!({
                "event": event,
                "actor": actor,
                "ts": ts.to_rfc3339(),
            });
            if let Some(label) = &e.label
                && let Some(entry_obj) = entry.as_object_mut()
            {
                entry_obj.insert(
                    "label".into(),
                    serde_json::Value::String(label.name.clone()),
                );
            }
            if let Some(body) = &e.body
                && let Some(entry_obj) = entry.as_object_mut()
            {
                entry_obj.insert("body".into(), serde_json::Value::String(body.clone()));
            }
            out.push(entry);
        }
        out.sort_by(|a, b| {
            let sa = a.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            let sb = b.get("ts").and_then(|v| v.as_str()).unwrap_or("");
            sa.cmp(sb)
        });
        Ok(out)
    }

    pub async fn post_story_comment(&self, number: u64, body: &str) -> Result<(), String> {
        self.client
            .issues(&self.owner, &self.repo)
            .create_comment(number, body)
            .await
            .map(|_| ())
            .map_err(Self::classify)
    }

    pub async fn list_stories(&self) -> Result<String, String> {
        let first = self
            .client
            .issues(&self.owner, &self.repo)
            .list()
            .state(octocrab::params::State::All)
            .per_page(100)
            .send()
            .await
            .map_err(Self::classify)?;
        let issues = self.client.all_pages(first).await.map_err(Self::classify)?;
        let condensed: Vec<serde_json::Value> = issues
            .iter()
            .map(|i| {
                serde_json::json!({
                    "number": i.number,
                    "title": i.title,
                    "state": match i.state {
                        octocrab::models::IssueState::Open => "OPEN",
                        octocrab::models::IssueState::Closed => "CLOSED",
                        _ => "OPEN",
                    },
                    "labels": i.labels.iter().map(|l| serde_json::json!({"name": l.name})).collect::<Vec<_>>(),
                })
            })
            .collect();
        serde_json::to_string(&condensed).map_err(|e| format!("serialize: {e}"))
    }

    pub async fn get_story_title(&self, number: u64) -> Option<String> {
        self.client
            .issues(&self.owner, &self.repo)
            .get(number)
            .await
            .ok()
            .map(|i| i.title)
    }
}
