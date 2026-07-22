use serde::Serialize;

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

impl From<&telemetry::tokens::TokenReport> for TokenReportView {
    fn from(report: &telemetry::tokens::TokenReport) -> Self {
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

impl From<&telemetry::tokens::TokenReportEntry> for TokenReportEntryView {
    fn from(entry: &telemetry::tokens::TokenReportEntry) -> Self {
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

impl From<&bookmark::Bookmark> for BookmarkView {
    fn from(b: &bookmark::Bookmark) -> Self {
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
