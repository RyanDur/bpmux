//! Bridge-owned response shapes — `bridge::view_model::*` types are
//! the public-API shapes returned by bridge tools to MCP callers.
//!
//! Per architecture.md Friction #2 (#1363): bridge tools previously
//! serialized `session::*` types directly via their `Serialize`
//! derive, leaking the session schema across the bridge↔MCP wire. A
//! field rename in `session/identity.rs` would silently change the
//! MCP JSON shape — making bridge a downstream of session refactors.
//!
//! The view-model boundary inverts that dependency: bridge owns its
//! response shapes, and per-class `From<&session::Foo>` translations
//! decide what to expose. Session schema changes are absorbed by the
//! translation step, not propagated to MCP consumers.
//!
//! This is the foundation slice — slice 1 of the multi-slice
//! migration. Subsequent slices populate the module with views for
//! each bridge-tool class (identity write-path / briefing / tokens /
//! bookmark / messages / etc.).

use serde::Serialize;

use crate::session;

/// Mirror of `session::identity::IdentityValue` exposed at the
/// bridge↔MCP boundary. Field-for-field today; the boundary's value
/// lands when session::IdentityValue gains/renames fields — the
/// translation in `From` controls what flows to MCP.
#[derive(Debug, Clone, Serialize)]
pub struct IdentityValueView {
    pub name: String,
    pub description: String,
    pub principles: Vec<String>,
}

impl From<&session::identity::IdentityValue> for IdentityValueView {
    fn from(value: &session::identity::IdentityValue) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            principles: value.principles.clone(),
        }
    }
}

/// Mirror of `session::identity::IdentityIndex` — the collection
/// of identity values for one role.
#[derive(Debug, Clone, Serialize, Default)]
pub struct IdentityIndexView {
    pub values: Vec<IdentityValueView>,
}

impl From<&session::identity::IdentityIndex> for IdentityIndexView {
    fn from(index: &session::identity::IdentityIndex) -> Self {
        Self {
            values: index.values.iter().map(IdentityValueView::from).collect(),
        }
    }
}

/// Bridge-owned mirror of `session::tokens::TokenReport`.
#[derive(Debug, Clone, Serialize)]
pub struct TokenReportView {
    pub total_tokens: u64,
    pub row_count: usize,
    pub by_role: std::collections::BTreeMap<String, u64>,
    pub recent: Vec<TokenReportEntryView>,
}

/// Bridge-owned mirror of `session::tokens::TokenReportEntry`.
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

impl From<&session::tokens::TokenReport> for TokenReportView {
    fn from(report: &session::tokens::TokenReport) -> Self {
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

impl From<&session::tokens::TokenReportEntry> for TokenReportEntryView {
    fn from(entry: &session::tokens::TokenReportEntry) -> Self {
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

/// Bridge-owned mirror of `session::bookmark::Bookmark`. The bookmark
/// is per-role mid-session-state structured-snapshot the agent
/// curates for post-compact re-entry context.
#[derive(Debug, Clone, Serialize, Default)]
pub struct BookmarkView {
    pub current_task: String,
    pub decisions: Vec<String>,
    pub open_questions: Vec<String>,
    pub next_step: String,
    pub notes: String,
    pub last_updated: String,
}

impl From<&session::bookmark::Bookmark> for BookmarkView {
    fn from(b: &session::bookmark::Bookmark) -> Self {
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

/// Bridge-owned mirror of `session::velocity::VelocityMetrics`. The
/// renderer-side displays these without re-deriving; the agent-side
/// `view_velocity` tool returns the same shape.
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

impl From<&session::velocity::VelocityMetrics> for VelocityMetricsView {
    fn from(m: &session::velocity::VelocityMetrics) -> Self {
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
        let src = session::identity::IdentityValue {
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
        let src = session::identity::IdentityIndex {
            values: vec![
                session::identity::IdentityValue {
                    name: "alpha".to_string(),
                    description: "a".to_string(),
                    principles: vec![],
                },
                session::identity::IdentityValue {
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
        let src = session::identity::IdentityIndex {
            values: vec![session::identity::IdentityValue {
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
