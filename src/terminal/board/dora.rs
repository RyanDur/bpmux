/// Story-throughput metrics inspired by DORA framing. Each Option
/// is None when data is too sparse to be load-bearing — honesty over
/// false confidence, same pattern as `session::velocity`.
pub struct DoraMetrics {
    pub shipped_this_week: usize,
    pub in_progress: usize,
    pub rejection_rate: Option<f64>,
    pub avg_cycle_hours: Option<f64>,
    pub completed_count: usize,
}

/// Sparse threshold below which medians/rates aren't load-bearing.
/// Matches `session::velocity::SPARSE_THRESHOLD`.
const DORA_SPARSE_THRESHOLD: usize = 3;

impl DoraMetrics {
    pub fn format(&self) -> String {
        let mut lines = vec!["## Metrics (DORA)".to_string(), String::new()];
        lines.push(format!("  Shipped this week:  {}", self.shipped_this_week));
        lines.push(format!("  In progress:        {}", self.in_progress));
        match self.rejection_rate {
            Some(rate) => lines.push(format!("  Rejection rate:     {rate:.1}%")),
            None => lines.push(format!(
                "  Rejection rate:     insufficient data ({}/{} completed)",
                self.completed_count, DORA_SPARSE_THRESHOLD
            )),
        }
        match self.avg_cycle_hours {
            Some(hours) if hours < 1.0 => {
                lines.push(format!("  Avg cycle time:     {:.0} min", hours * 60.0))
            }
            Some(hours) => lines.push(format!("  Avg cycle time:     {hours:.1} hrs")),
            None => lines.push(format!(
                "  Avg cycle time:     insufficient data ({}/{} accepted)",
                self.completed_count, DORA_SPARSE_THRESHOLD
            )),
        }
        lines.join("\n")
    }
}

pub(in crate::terminal) fn fetch_dora_metrics() -> Option<DoraMetrics> {
    let mut cmd = std::process::Command::new("gh");
    cmd.args([
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "200",
        "--json",
        "state,labels,title,createdAt,closedAt",
    ]);
    if let Ok(repo) = std::env::var("BPMUX_REPO") {
        cmd.args(["-R", &repo]);
    } else {
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"));
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let issues: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).ok()?;
    Some(compute_dora_metrics(&issues, chrono::Utc::now()))
}

/// Pure compute over fetched gh issue data. Extracted from
/// `fetch_dora_metrics` so the bug-fixed math is testable without
/// shelling out. See `DoraMetrics` for the field semantics.
///
/// **Accuracy bug-fixes** (#218):
/// - Cycle time + `shipped_this_week` only count ACCEPTED + FEATURE
///   stories per the #264 Feature/Bug/Chore vocabulary. Previously
///   counted every accept, which let Chores and Bugs inflate the
///   "shipped this week" number — Ryan caught this as "shipped this
///   week: 100" when the repo has nowhere near that many Feature
///   accepts in a week. Business-value velocity = Feature-only.
/// - Cycle time is `None` when fewer than `DORA_SPARSE_THRESHOLD`
///   accepted Features exist. Previously returned `0.0` on empty,
///   which renders as "0 min" — false signal of fast cycle.
/// - Rejection rate uses (accepted + rejected) as denominator.
///   Previously used total_closed, which includes deferred/abandoned
///   stories — those aren't rejections.
/// - Rejection rate is `None` when fewer than `DORA_SPARSE_THRESHOLD`
///   completed stories exist.
pub fn compute_dora_metrics(
    issues: &[serde_json::Value],
    now: chrono::DateTime<chrono::Utc>,
) -> DoraMetrics {
    let week_ago = now - chrono::Duration::days(7);

    let mut shipped_this_week = 0usize;
    let mut in_progress = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut cycle_times: Vec<f64> = Vec::new();

    for issue in issues {
        let state = issue["state"].as_str().unwrap_or("OPEN");
        let labels: Vec<String> = issue["labels"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|l| l["name"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let title = issue["title"].as_str().unwrap_or("");
        // #218: DORA's velocity-like numbers ("shipped this week",
        // cycle time) must match the #264 Feature/Bug/Chore vocabulary
        // so Chore accepts don't inflate the count (the "shipped this
        // week: 100" inflation Ryan caught). Rejection rate and
        // in-progress stay label-only because they're workflow-state
        // counts, not velocity.
        let is_feature = crate::session::velocity::is_feature_verdict(&labels, title);
        let has_label = |name: &str| labels.iter().any(|l| l == name);

        if state == "CLOSED" {
            let is_rejected = has_label("rejected");
            let is_deferred = has_label("deferred");
            let is_accepted = !is_rejected && !is_deferred;

            if is_accepted {
                accepted += 1;
            }
            if is_rejected {
                rejected += 1;
            }

            if let (Some(created), Some(closed)) = (
                issue["createdAt"]
                    .as_str()
                    .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
                issue["closedAt"]
                    .as_str()
                    .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok()),
            ) {
                if is_accepted && is_feature && closed > week_ago {
                    shipped_this_week += 1;
                }
                if is_accepted && is_feature {
                    let hours = (closed - created).num_minutes() as f64 / 60.0;
                    if hours >= 0.0 {
                        cycle_times.push(hours);
                    }
                }
            }
        } else if has_label("started") || has_label("finished") {
            in_progress += 1;
        }
    }

    let completed_count = accepted + rejected;
    let rejection_rate = if completed_count >= DORA_SPARSE_THRESHOLD {
        Some(rejected as f64 / completed_count as f64 * 100.0)
    } else {
        None
    };

    let avg_cycle_hours = if cycle_times.len() >= DORA_SPARSE_THRESHOLD {
        Some(cycle_times.iter().sum::<f64>() / cycle_times.len() as f64)
    } else {
        None
    };

    DoraMetrics {
        shipped_this_week,
        in_progress,
        rejection_rate,
        avg_cycle_hours,
        completed_count,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    fn now() -> chrono::DateTime<chrono::Utc> {
        "2026-04-17T12:00:00Z".parse().unwrap()
    }

    fn issue(
        state: &str,
        labels: &[&str],
        title: &str,
        closed_ago_days: Option<i64>,
    ) -> serde_json::Value {
        let created = now() - Duration::days(10);
        let closed = closed_ago_days.map(|d| now() - Duration::days(d));
        json!({
            "state": state,
            "labels": labels.iter().map(|n| json!({"name": n})).collect::<Vec<_>>(),
            "title": title,
            "createdAt": created.to_rfc3339(),
            "closedAt": closed.map(|c| c.to_rfc3339()),
        })
    }

    #[test]
    fn shipped_this_week_only_counts_feature_accepts() {
        // Ryan's "shipped this week: 100" inflation: Chore + Bug
        // closes were being counted as if they were shipped features.
        let issues = vec![
            issue("CLOSED", &["Feature"], "feature A", Some(2)),
            issue("CLOSED", &["Feature"], "feature B", Some(3)),
            issue("CLOSED", &["Chore"], "[Chore] refactor", Some(1)),
            issue("CLOSED", &["Bug"], "[Bug] fix", Some(1)),
        ];
        let m = compute_dora_metrics(&issues, now());
        assert_eq!(
            m.shipped_this_week, 2,
            "shipped_this_week counts only Feature accepts"
        );
    }

    #[test]
    fn shipped_this_week_excludes_rejected_and_deferred_features() {
        let issues = vec![
            issue("CLOSED", &["Feature"], "kept", Some(2)),
            issue(
                "CLOSED",
                &["Feature", "rejected"],
                "rejected feature",
                Some(2),
            ),
            issue(
                "CLOSED",
                &["Feature", "deferred"],
                "deferred feature",
                Some(2),
            ),
        ];
        let m = compute_dora_metrics(&issues, now());
        assert_eq!(m.shipped_this_week, 1);
    }

    #[test]
    fn shipped_this_week_counts_bracketed_title_as_feature() {
        // No label yet (backfill lag) — title prefix carries intent.
        let issues = vec![
            issue("CLOSED", &[], "[Feature] fresh story", Some(1)),
            issue("CLOSED", &[], "[Chore] cleanup", Some(1)),
        ];
        let m = compute_dora_metrics(&issues, now());
        assert_eq!(m.shipped_this_week, 1);
    }

    #[test]
    fn cycle_time_excludes_non_features() {
        // Three Chores accepted very quickly shouldn't drive the
        // "Feature cycle time" number down — that would mask slow
        // business-value delivery.
        let issues = vec![
            issue("CLOSED", &["Chore"], "[Chore] a", Some(1)),
            issue("CLOSED", &["Chore"], "[Chore] b", Some(1)),
            issue("CLOSED", &["Chore"], "[Chore] c", Some(1)),
        ];
        let m = compute_dora_metrics(&issues, now());
        assert!(
            m.avg_cycle_hours.is_none(),
            "below-threshold Features → None, not a Chore-derived number"
        );
    }

    #[test]
    fn in_progress_counts_started_or_finished_regardless_of_type() {
        // In-progress is workflow state, not velocity — Chores and
        // Bugs in flight still count as in-progress work.
        let issues = vec![
            issue("OPEN", &["started"], "[Feature] w1", None),
            issue("OPEN", &["finished"], "[Chore] w2", None),
            issue("OPEN", &["started"], "[Bug] w3", None),
        ];
        let m = compute_dora_metrics(&issues, now());
        assert_eq!(m.in_progress, 3);
    }
}
