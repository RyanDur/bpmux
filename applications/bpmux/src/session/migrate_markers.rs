//! #1301 boot-time migration of markers-store entries into iteration-issues.
//!
//! Slice 5 of Epic #1189 architectural-pivot. Runs after the existing boot-
//! heal chain (#1268/#1271/#1283/#1288/#1294) once-per-boot. For each
//! `IterationMarker` in markers-store, builds a `MigrationPlan` that either:
//!
//! - SKIPS the marker (idempotent guard): an open iteration-issue
//!   (`story_type=Iteration`) with a matching `Iteration {theme}` title
//!   already exists in the canonical store — second-boot post-migration
//!   converges on no-op.
//! - CREATES a new iteration-issue for the marker: when no matching open
//!   issue exists. Duplicate themes (e.g., two markers both named
//!   "Aquila") are disambiguated deterministically: markers are sorted
//!   by `position` ascending; the first keeps the bare theme, subsequent
//!   ones get suffixes " B" / " C" / etc.
//!
//! The execution step fires `refresh.create_story` for each Create plan,
//! then for each member-story listed in `marker.stories[]` it fires
//! `BoardEvent::TagAdded("iteration-#{new_n}")` via mutate_sync — the
//! slice-2 label decoder + #1299 BoardStore::apply sync-helpers populate
//! `Story.iteration_id` from this label. State-delta broadcast names
//! the migration count.
//!
//! markers-store is **not** deleted by this slice — slice 6 #1302
//! close-out retires the markers-store + icebox_unified marker-substrate
//! once the renderer transitions to iteration-issue grouping. Strangler-
//! fig discipline matches #1299 + #1300 parallel-substrate transition.

use std::sync::Arc;

use crate::board::events::EventMeta;
use crate::board::{BoardEvent, PeerId, RefreshDriver, Story};
use crate::session::iteration_markers::IterationMarker;

/// A single planned migration action for one markers-store entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationPlan {
    /// An iteration-issue with this theme already exists (matched by
    /// `Iteration {theme}` title, story_type=Iteration, not-closed).
    /// Idempotent guard: the canonical substrate already reflects this
    /// marker; skip creation. Member-stories' iteration_id is assumed
    /// already populated (or will be repaired on subsequent
    /// add_stories_to_iteration runs).
    Skip { theme: String, issue_number: u64 },
    /// No matching iteration-issue exists; create one and label the
    /// listed member-stories so the slice-2 decoder populates their
    /// iteration_id. `title` carries the deduplicated form (e.g.,
    /// "Iteration Aquila B" for the second of two same-theme markers).
    Create {
        marker_id: u64,
        title: String,
        members: Vec<u64>,
    },
}

/// Build the migration plan from current markers-store state and the
/// current canonical store. Pure function — does not perform any I/O —
/// so the planner is unit-testable in isolation from the orchestrator.
pub fn plan_marker_migration(
    markers: &[IterationMarker],
    existing_iteration_issues: &[Story],
) -> Vec<MigrationPlan> {
    use std::collections::HashMap;

    let mut existing_by_title: HashMap<String, u64> = HashMap::new();
    for issue in existing_iteration_issues {
        existing_by_title.insert(issue.title.to_ascii_lowercase(), issue.number);
    }

    let mut markers_sorted: Vec<&IterationMarker> = markers.iter().collect();
    markers_sorted.sort_by_key(|m| m.position);

    let mut theme_counts: HashMap<String, usize> = HashMap::new();
    let mut plans: Vec<MigrationPlan> = Vec::new();

    for marker in markers_sorted {
        let theme_key = marker.theme_name.to_ascii_lowercase();
        let n_seen = theme_counts.entry(theme_key.clone()).or_insert(0);
        let title = if *n_seen == 0 {
            format!("Iteration {}", marker.theme_name)
        } else {
            let suffix = letter_suffix(*n_seen);
            format!("Iteration {} {suffix}", marker.theme_name)
        };
        *n_seen += 1;

        let title_key = title.to_ascii_lowercase();
        if let Some(issue_number) = existing_by_title.get(&title_key).copied() {
            plans.push(MigrationPlan::Skip {
                theme: marker.theme_name.clone(),
                issue_number,
            });
        } else {
            plans.push(MigrationPlan::Create {
                marker_id: marker.id,
                title,
                members: marker.stories.clone(),
            });
        }
    }
    plans
}

/// Deterministic letter suffix for duplicate-theme disambiguation.
/// Sequence: 0 -> (no suffix), 1 -> "B", 2 -> "C", ..., 25 -> "Z",
/// 26 -> "AA" (graceful overflow; not expected in practice).
fn letter_suffix(n_seen: usize) -> String {
    let mut buf = Vec::new();
    let mut x = n_seen;
    loop {
        buf.push(b'A' + (x % 26) as u8);
        x /= 26;
        if x == 0 {
            break;
        }
        x -= 1;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap_or_else(|_| "X".to_string())
}

/// Result of `execute_marker_migration` summarizing what changed.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MigrationOutcome {
    pub created: Vec<(u64, String, u64)>,
    pub skipped: Vec<(String, u64)>,
    pub errors: Vec<String>,
}

impl MigrationOutcome {
    pub fn migrated_count(&self) -> usize {
        self.created.len()
    }
}

/// Execute the migration plan. For each `Create` plan: fires
/// `refresh.create_story("Iteration {theme}", body)` to mint the
/// iteration-issue, then fires `BoardEvent::TagAdded("iteration",
/// number=new_n)` so the issue carries the `iteration` label (sister to
/// `handle_create_iteration` at the canonical-chain axis), then for each
/// member-story fires `BoardEvent::TagAdded("iteration-#{new_n}")` so
/// the slice-2 label decoder populates `Story.iteration_id`. Errors are
/// collected best-effort (one failure doesn't abort the batch).
pub async fn execute_marker_migration(
    refresh: &Arc<dyn RefreshDriver>,
    plan: Vec<MigrationPlan>,
) -> MigrationOutcome {
    let mut outcome = MigrationOutcome::default();
    let meta = || EventMeta {
        actor: "pm".to_string(),
        ts: chrono::Utc::now(),
        peer_id: PeerId::local(),
    };
    for action in plan {
        match action {
            MigrationPlan::Skip {
                theme,
                issue_number,
            } => {
                outcome.skipped.push((theme, issue_number));
            }
            MigrationPlan::Create {
                marker_id,
                title,
                members,
            } => {
                let body = compose_migration_body(&title);
                match refresh.create_story(title.clone(), body.clone()).await {
                    Ok(new_n) => {
                        let _ = refresh.mutate_sync(BoardEvent::StoryCreated {
                            meta: meta(),
                            number: new_n,
                            title: title.clone(),
                            body: body.clone(),
                            initial_state: crate::board::domain::StoryState::Icebox,
                            story_type: crate::board::domain::StoryType::Iteration,
                        });
                        let _ = refresh.mutate_sync(BoardEvent::TagAdded {
                            meta: meta(),
                            number: new_n,
                            label: "iteration".to_string(),
                        });
                        let _ = refresh.mutate_sync(BoardEvent::TagAdded {
                            meta: meta(),
                            number: new_n,
                            label: "icebox".to_string(),
                        });
                        let label = format!("iteration-#{new_n}");
                        for member in &members {
                            if let Err(e) = refresh.mutate_sync(BoardEvent::TagAdded {
                                meta: meta(),
                                number: *member,
                                label: label.clone(),
                            }) {
                                outcome.errors.push(format!(
                                    "marker-id={marker_id} member-tag #{member} -> {label}: {e}"
                                ));
                            }
                        }
                        outcome.created.push((marker_id, title, new_n));
                    }
                    Err(e) => {
                        outcome.errors.push(format!(
                            "marker-id={marker_id} create_story({title}) failed: {e}"
                        ));
                    }
                }
            }
        }
    }
    outcome
}

fn compose_migration_body(title: &str) -> String {
    format!(
        "# {title}\n\n\
         Created via #1301 boot-time markers-store migration (Epic #1189 slice 5).\n\n\
         ## Status\n\nOpen\n\n## Member stories\n\n(populated via iteration-#NNNN labels per slice-2 substrate)\n\n## Notes\n\nClose via the `close_iteration` ceremony (slice 4 #1300; canonical-via-GH path).\n"
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{StoryState, StoryType};
    use crate::board::orchestrator::SyncOrchestrator;
    use crate::board::store::BoardStore;
    use chrono::{TimeZone, Utc};

    fn marker(id: u64, theme: &str, position: u32, stories: Vec<u64>) -> IterationMarker {
        IterationMarker {
            id,
            theme_name: theme.to_string(),
            position,
            created_at: "2026-05-12T00:00:00Z".to_string(),
            stories,
        }
    }

    fn iteration_issue(n: u64, title: &str) -> Story {
        let mut s = Story::new_minimal(
            n,
            title.to_string(),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.story_type = StoryType::Iteration;
        s.raw_labels = vec!["iteration".to_string()];
        s
    }

    async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch = SyncOrchestrator::new(store, adapter);
        let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
        orch.refresh_all().await.unwrap();
        orch
    }

    #[test]
    fn plan_creates_iteration_issue_for_each_unseen_marker() {
        let markers = vec![marker(2, "Aquila", 0, vec![100, 200])];
        let plans = plan_marker_migration(&markers, &[]);
        assert_eq!(plans.len(), 1);
        assert!(matches!(
            &plans[0],
            MigrationPlan::Create { title, members, .. }
                if title == "Iteration Aquila" && members == &vec![100, 200]
        ));
    }

    #[test]
    fn plan_disambiguates_duplicate_themes_by_position() {
        let markers = vec![
            marker(5, "Aquila", 1, vec![300]),
            marker(2, "Aquila", 0, vec![100]),
        ];
        let plans = plan_marker_migration(&markers, &[]);
        assert_eq!(plans.len(), 2);
        if let MigrationPlan::Create { title, .. } = &plans[0] {
            assert_eq!(title, "Iteration Aquila", "first-position keeps bare theme");
        } else {
            panic!("expected Create at index 0: {:?}", plans[0]);
        }
        if let MigrationPlan::Create { title, .. } = &plans[1] {
            assert_eq!(title, "Iteration Aquila B", "second gets B suffix");
        } else {
            panic!("expected Create at index 1: {:?}", plans[1]);
        }
    }

    #[test]
    fn plan_skips_marker_when_existing_iteration_issue_matches_title() {
        let markers = vec![marker(2, "Aquila", 0, vec![100])];
        let existing = vec![iteration_issue(1297, "Iteration Aquila")];
        let plans = plan_marker_migration(&markers, &existing);
        assert!(matches!(
            &plans[0],
            MigrationPlan::Skip { theme, issue_number }
                if theme == "Aquila" && *issue_number == 1297
        ));
    }

    #[test]
    fn plan_is_idempotent_across_multiple_runs_with_same_inputs() {
        // First run produces all-Create plans.
        let markers = vec![
            marker(2, "Aquila", 0, vec![100]),
            marker(6, "Pegasus", 1, vec![200]),
        ];
        let first = plan_marker_migration(&markers, &[]);
        assert!(
            first
                .iter()
                .all(|p| matches!(p, MigrationPlan::Create { .. }))
        );
        // After execution, both iteration-issues exist; second-run plan
        // produces all-Skip (idempotent guard).
        let after_execution = vec![
            iteration_issue(1297, "Iteration Aquila"),
            iteration_issue(1298, "Iteration Pegasus"),
        ];
        let second = plan_marker_migration(&markers, &after_execution);
        assert!(
            second
                .iter()
                .all(|p| matches!(p, MigrationPlan::Skip { .. }))
        );
    }

    #[tokio::test]
    async fn execute_creates_iteration_issue_and_labels_members() {
        let markers = vec![marker(2, "Aquila", 0, vec![100, 200])];
        let refresh = build_refresh(vec![
            Story::new_minimal(
                100,
                "[Feature] member-100".to_string(),
                StoryState::Icebox,
                Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
                "pm".to_string(),
            ),
            Story::new_minimal(
                200,
                "[Feature] member-200".to_string(),
                StoryState::Icebox,
                Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
                "pm".to_string(),
            ),
        ])
        .await;
        let plan = plan_marker_migration(&markers, &[]);
        let outcome = execute_marker_migration(&refresh, plan).await;
        assert_eq!(outcome.migrated_count(), 1, "1 iteration-issue created");
        assert_eq!(outcome.errors.len(), 0);
        let (_marker_id, title, new_n) = &outcome.created[0];
        assert_eq!(title, "Iteration Aquila");
        let iter_issue = refresh.story(*new_n).expect("iteration-issue persisted");
        assert!(matches!(iter_issue.story_type, StoryType::Iteration));
        let s100 = refresh.story(100).expect("member 100 exists");
        assert_eq!(s100.iteration_id, Some(*new_n), "member 100 iteration_id");
        let s200 = refresh.story(200).expect("member 200 exists");
        assert_eq!(s200.iteration_id, Some(*new_n), "member 200 iteration_id");
    }

    #[test]
    fn letter_suffix_sequence() {
        assert_eq!(letter_suffix(1), "B");
        assert_eq!(letter_suffix(2), "C");
        assert_eq!(letter_suffix(25), "Z");
        assert_eq!(letter_suffix(26), "AA");
    }
}
