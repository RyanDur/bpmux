use crate::product_management::roadmap::selectors::{OutcomeState, RoadmapOutcomeCard};
use crate::product_management::story::Horizon;

pub fn render_overview_layer(value: &serde_json::Value) -> String {
    let outcomes: Vec<RoadmapOutcomeCard> =
        serde_json::from_value(value["outcomes"].clone()).unwrap_or_default();
    let mut out = render_roadmap(&outcomes);
    let total = value["total"].as_u64().unwrap_or(0);
    let shown = value["shown"].as_u64().unwrap_or(total);
    if total > shown {
        out.push_str(&format!("showing {shown} of {total}\n"));
    }
    out
}

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
            card.id.value(),
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
        OutcomeState::NeedsAttention => format!("{} needs-attention", card.gap_count),
        OutcomeState::Planned => "planned · iterations TBD".to_string(),
    }
}
