use std::sync::Arc;

pub(crate) fn ordered_board_stories(
    refresh: &Arc<dyn crate::board::RefreshDriver>,
) -> Vec<serde_json::Value> {
    let all = refresh.all_stories();
    let codec = crate::board::adapters::gh_label_codec::GhLabelCodec;
    let workflow = crate::workflow::current_workflow();
    let stories: Vec<serde_json::Value> = all
        .iter()
        .map(|s| {
            let mut json = crate::board::adapters::json::story_to_board_json(&codec, s);
            if s.story_type == crate::board::domain::StoryType::Iteration
                && let serde_json::Value::Object(map) = &mut json
            {
                let members: Vec<&crate::board::domain::Story> = all
                    .iter()
                    .filter(|m| m.iteration_membership.map(|(it, _)| it) == Some(s.number))
                    .collect();
                let readiness =
                    crate::board::iteration_readiness::iteration_readiness(s, &members, workflow);
                if let Ok(value) = serde_json::to_value(readiness) {
                    map.insert("readiness".to_string(), value);
                }
            }
            if s.story_type == crate::board::domain::StoryType::Outcome
                && let serde_json::Value::Object(map) = &mut json
            {
                let rolled =
                    crate::board::iteration_readiness::outcome_readiness(s, &all, workflow);
                if let Ok(value) = serde_json::to_value(rolled) {
                    map.insert("outcome_readiness".to_string(), value);
                }
            }
            json
        })
        .collect();
    crate::board::ready_order_label::sort_board_ready_by_membership(stories)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::ordered_board_stories;
    use crate::board::RefreshDriver;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use crate::board::{BoardStore, SyncOrchestrator};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn orchestrator(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
        orch.refresh_all().await.unwrap();
        orch
    }

    fn iteration(number: u64) -> Story {
        let mut s = Story::new_minimal(
            number,
            "Iteration Vela".to_string(),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
            "pm".into(),
        );
        s.story_type = StoryType::Iteration;
        s.body = Some("## Iteration overview\n\ncomposed".to_string());
        s.iteration_review_closed_by = vec!["designer".to_string()];
        s.iteration_dev_ok_by = vec!["dev".to_string()];
        s.raw_labels = vec![
            "iteration".into(),
            "ready".into(),
            "iteration-review-closed.designer".into(),
            "iteration-dev-ok.dev".into(),
        ];
        s
    }

    fn member(number: u64, iteration_number: u64, position: u32) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("member #{number}"),
            StoryState::Ready,
            Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
            "pm".into(),
        );
        s.iteration_membership = Some((iteration_number, position));
        s.raw_labels = vec![format!("parent-iteration-#{iteration_number}.{position}")];
        s
    }

    fn find(stories: &[serde_json::Value], number: u64) -> &serde_json::Value {
        stories
            .iter()
            .find(|s| s["number"].as_u64() == Some(number))
            .expect("story present in snapshot")
    }

    fn outcome(number: u64) -> Story {
        let mut s = Story::new_minimal(
            number,
            "An outcome".to_string(),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
            "pm".into(),
        );
        s.story_type = StoryType::Outcome;
        s.raw_labels = vec!["outcome".into(), "horizon.now".into()];
        s
    }

    fn building_iteration(number: u64, outcome_number: u64, position: u32) -> Story {
        let mut s = iteration(number);
        s.outcome_membership = Some((outcome_number, position));
        s.raw_labels
            .push(format!("parent-outcome-#{outcome_number}.{position}"));
        s
    }

    #[tokio::test]
    async fn outcome_snapshot_carries_rolled_up_readiness_1738() {
        let orch = orchestrator(vec![
            outcome(3000),
            building_iteration(3010, 3000, 1),
            member(3011, 3010, 100),
        ])
        .await;
        let stories = ordered_board_stories(&orch);
        let rolled = &find(&stories, 3000)["outcome_readiness"];
        assert_eq!(
            rolled["iteration_count"].as_u64(),
            Some(1),
            "one iteration builds the outcome"
        );
        assert_eq!(rolled["gap_count"].as_u64(), Some(0));
        assert_eq!(
            rolled["all_ready"].as_bool(),
            Some(true),
            "the building iteration is fully ready → the outcome's readiness rolls up to ready, exposed for #1704/#1737"
        );
    }

    #[tokio::test]
    async fn iteration_snapshot_carries_computed_readiness_1703() {
        let orch = orchestrator(vec![iteration(2000), member(2001, 2000, 100)]).await;
        let stories = ordered_board_stories(&orch);
        let readiness = &find(&stories, 2000)["readiness"];
        assert_eq!(
            readiness["review_closed"].as_bool(),
            Some(true),
            "the designer review-close derives review_closed=true, exposed for #1704 to consume"
        );
        assert_eq!(readiness["drafted"].as_bool(), Some(true));
        assert_eq!(readiness["stories_ordered"].as_bool(), Some(true));
        assert_eq!(
            readiness["dev_ok"].as_bool(),
            Some(true),
            "the dev-OK derives dev_ok=true, exposed for #1704 to consume"
        );
    }

    #[tokio::test]
    async fn non_iteration_story_has_no_readiness_key_1703() {
        let orch = orchestrator(vec![member(2001, 2000, 100)]).await;
        let stories = ordered_board_stories(&orch);
        assert!(
            find(&stories, 2001).get("readiness").is_none(),
            "readiness is an iteration-only concern; feature stories carry no readiness key"
        );
    }
}
