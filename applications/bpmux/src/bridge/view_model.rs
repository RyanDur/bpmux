use serde::Serialize;

use crate::memory;

#[derive(Debug, Clone, Serialize)]
pub struct IdentityValueView {
    pub name: String,
    pub description: String,
    pub principles: Vec<String>,
}

impl From<&memory::identity::IdentityValue> for IdentityValueView {
    fn from(value: &memory::identity::IdentityValue) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            principles: value.principles.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct IdentityIndexView {
    pub values: Vec<IdentityValueView>,
}

impl From<&memory::identity::IdentityIndex> for IdentityIndexView {
    fn from(index: &memory::identity::IdentityIndex) -> Self {
        Self {
            values: index.values.iter().map(IdentityValueView::from).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenReportView {
    pub total_tokens: u64,
    pub row_count: usize,
    pub by_role: std::collections::BTreeMap<String, u64>,
    pub recent: Vec<TokenReportEntryView>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TokenReportEntryView {
    pub ts: String,
    pub role: String,
    pub harness: String,
    pub model: String,
    pub total_tokens: u64,
    pub output_tokens: u64,
    pub tool_call_count: u32,
}

impl From<&crate::telemetry::tokens::TokenReport> for TokenReportView {
    fn from(report: &crate::telemetry::tokens::TokenReport) -> Self {
        Self {
            total_tokens: report.total_tokens,
            row_count: report.row_count,
            by_role: report.by_role.clone(),
            recent: report
                .recent
                .iter()
                .map(TokenReportEntryView::from)
                .collect(),
        }
    }
}

impl From<&crate::telemetry::tokens::TokenReportEntry> for TokenReportEntryView {
    fn from(entry: &crate::telemetry::tokens::TokenReportEntry) -> Self {
        Self {
            ts: entry.ts.clone(),
            role: entry.role.clone(),
            harness: entry.harness.clone(),
            model: entry.model.clone(),
            total_tokens: entry.total_tokens,
            output_tokens: entry.output_tokens,
            tool_call_count: entry.tool_call_count,
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct BookmarkView {
    pub current_task: String,
    pub decisions: Vec<String>,
    pub open_questions: Vec<String>,
    pub next_step: String,
    pub notes: String,
    pub last_updated: String,
}

impl From<&crate::bookmark::Bookmark> for BookmarkView {
    fn from(b: &crate::bookmark::Bookmark) -> Self {
        Self {
            current_task: b.current_task.clone(),
            decisions: b.decisions.clone(),
            open_questions: b.open_questions.clone(),
            next_step: b.next_step.clone(),
            notes: b.notes.clone(),
            last_updated: b.last_updated.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct VelocityMetricsView {
    pub window_days: i64,
    pub delivered_in_window: usize,
    pub delivered_per_day: f64,
    pub median_pickup_to_deliver_hours: Option<f64>,
    pub median_deliver_to_accept_hours: Option<f64>,
    pub rework_rate: Option<f64>,
    pub sparse: bool,
    pub note: Option<String>,
}

impl From<&crate::velocity::VelocityMetrics> for VelocityMetricsView {
    fn from(m: &crate::velocity::VelocityMetrics) -> Self {
        Self {
            window_days: m.window_days,
            delivered_in_window: m.delivered_in_window,
            delivered_per_day: m.delivered_per_day,
            median_pickup_to_deliver_hours: m.median_pickup_to_deliver_hours,
            median_deliver_to_accept_hours: m.median_deliver_to_accept_hours,
            rework_rate: m.rework_rate,
            sparse: m.sparse,
            note: m.note.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_value_view_mirrors_source_fields() {
        let src = memory::identity::IdentityValue {
            name: "iteration over prediction".to_string(),
            description: "ship and learn".to_string(),
            principles: vec!["small steps".to_string()],
        };
        let view = IdentityValueView::from(&src);
        assert_eq!(view.name, "iteration over prediction");
        assert_eq!(view.description, "ship and learn");
        assert_eq!(view.principles, vec!["small steps".to_string()]);
    }

    #[test]
    fn identity_index_view_translates_full_collection() {
        let src = memory::identity::IdentityIndex {
            values: vec![
                memory::identity::IdentityValue {
                    name: "alpha".to_string(),
                    description: "a".to_string(),
                    principles: vec![],
                },
                memory::identity::IdentityValue {
                    name: "beta".to_string(),
                    description: "b".to_string(),
                    principles: vec!["p1".to_string(), "p2".to_string()],
                },
            ],
        };
        let view = IdentityIndexView::from(&src);
        assert_eq!(view.values.len(), 2);
        assert_eq!(view.values[0].name, "alpha");
        assert_eq!(view.values[1].principles.len(), 2);
    }

    #[test]
    fn identity_index_view_serializes_to_stable_json_shape() {
        let src = memory::identity::IdentityIndex {
            values: vec![memory::identity::IdentityValue {
                name: "iteration".to_string(),
                description: "ship".to_string(),
                principles: vec!["small".to_string()],
            }],
        };
        let view = IdentityIndexView::from(&src);
        let json = serde_json::to_string(&view).unwrap_or_default();
        assert!(!json.is_empty(), "view must serialize");
        assert!(json.contains("\"values\""));
        assert!(json.contains("\"name\":\"iteration\""));
        assert!(json.contains("\"description\":\"ship\""));
        assert!(json.contains("\"principles\":[\"small\"]"));
    }
}
