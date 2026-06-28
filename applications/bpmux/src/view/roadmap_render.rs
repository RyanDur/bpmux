use super::roadmap::{OutcomeState, RoadmapOutcomeCard};
use crate::product_management::story::Horizon;

pub fn render_roadmap(cards: &[RoadmapOutcomeCard]) -> String {
    let mut out = String::from("ROADMAP\n");
    if cards.is_empty() {
        out.push_str("  (no outcomes yet)\n");
        return out;
    }
    let mut current_horizon: Option<Horizon> = None;
    for card in cards {
        let band = if current_horizon == Some(card.horizon) {
            ""
        } else {
            current_horizon = Some(card.horizon);
            card.horizon.band_label()
        };
        out.push_str(&format!(
            "{band:<6} #{}  {}  {}\n",
            card.number,
            card.name,
            state_text(card)
        ));
    }
    out
}

fn state_text(card: &RoadmapOutcomeCard) -> String {
    match card.state {
        OutcomeState::Done => "done".to_string(),
        OutcomeState::Ready => "ready".to_string(),
        OutcomeState::NeedsAttention => {
            let count = card.gap_count;
            format!("{count} gap{}", if count == 1 { "" } else { "s" })
        }
        OutcomeState::Planned => "planned".to_string(),
    }
}
