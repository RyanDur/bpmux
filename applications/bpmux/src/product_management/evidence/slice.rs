use std::collections::BTreeMap;

use chrono::{DateTime, Utc};

use crate::product_management::story::StoryId;

use super::Evidence;

#[derive(Debug, Clone, Default)]
pub struct EvidenceState {
    pub stories: BTreeMap<StoryId, Vec<Evidence>>,
}

#[derive(Debug, Clone)]
pub enum EvidenceAction {
    VerdictSubmitted {
        story_id: StoryId,
        from_role: String,
        verdict: String,
        substrate_grounding: String,
        ts: DateTime<Utc>,
        grounded_head: Option<String>,
    },
}

pub fn reduce(state: &EvidenceState, action: &EvidenceAction) -> Option<EvidenceState> {
    match action {
        EvidenceAction::VerdictSubmitted {
            story_id,
            from_role,
            verdict,
            substrate_grounding,
            ts,
            grounded_head,
        } => {
            let entry = Evidence {
                story_id: *story_id,
                from_role: from_role.clone(),
                verdict: verdict.clone(),
                substrate_grounding: substrate_grounding.clone(),
                ts: *ts,
                grounded_head: grounded_head.clone(),
            };
            let mut stories = state.stories.clone();
            let list = stories.entry(*story_id).or_default();
            match list.iter_mut().find(|v| v.from_role == entry.from_role) {
                Some(existing) => *existing = entry,
                None => list.push(entry),
            }
            Some(EvidenceState { stories })
        }
    }
}

pub fn evidence_for_story(state: &EvidenceState, story_id: StoryId) -> Vec<Evidence> {
    state.stories.get(&story_id).cloned().unwrap_or_default()
}
