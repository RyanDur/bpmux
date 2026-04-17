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
/// - Cycle time only includes ACCEPTED stories (closed without
///   `rejected` or `deferred` labels). Previously included every
///   closed issue — a story that sat in icebox for months before
///   being deferred was averaged in, inflating cycle time wildly.
/// - Cycle time is `None` when fewer than `DORA_SPARSE_THRESHOLD`
///   accepted stories exist. Previously returned `0.0` on empty,
///   which renders as "0 min" — false signal of fast cycle.
/// - Rejection rate uses (accepted + rejected) as denominator.
///   Previously used total_closed, which includes deferred/abandoned
///   stories — those aren't rejections.
/// - Rejection rate is `None` when fewer than `DORA_SPARSE_THRESHOLD`
///   completed stories exist.
/// - `shipped_this_week` only counts ACCEPTED stories closed in the
///   last 7 days (not deferred/rejected). Matches the user intuition
///   for "shipped."
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
        let labels: Vec<&str> = issue["labels"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
            .unwrap_or_default();

        if state == "CLOSED" {
            let is_rejected = labels.contains(&"rejected");
            let is_deferred = labels.contains(&"deferred");
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
                if is_accepted && closed > week_ago {
                    shipped_this_week += 1;
                }
                if is_accepted {
                    let hours = (closed - created).num_minutes() as f64 / 60.0;
                    if hours >= 0.0 {
                        cycle_times.push(hours);
                    }
                }
            }
        } else if labels.contains(&"started") || labels.contains(&"finished") {
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
