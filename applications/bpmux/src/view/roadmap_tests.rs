#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::roadmap::*;
use super::roadmap_render::render_roadmap;
use crate::product_management::iteration::readiness::OutcomeReadiness;
use crate::product_management::story::Horizon;

fn outcome(number: u64, title: &str, horizon: Option<Horizon>) -> RoadmapOutcomeInput {
    RoadmapOutcomeInput {
        number,
        title: title.to_string(),
        body: String::new(),
        horizon,
        horizon_order: None,
        done: false,
        readiness: None,
    }
}

fn with_order(mut o: RoadmapOutcomeInput, order: Option<u32>) -> RoadmapOutcomeInput {
    o.horizon_order = order;
    o
}

fn readiness(iteration_count: u32, gap_count: u32, all_ready: bool) -> OutcomeReadiness {
    OutcomeReadiness {
        iteration_count,
        gap_count,
        all_ready,
    }
}

fn card_for(
    done: bool,
    readiness: Option<OutcomeReadiness>,
    horizon: Horizon,
) -> RoadmapOutcomeCard {
    let mut o = outcome(1, "An outcome", Some(horizon));
    o.done = done;
    o.readiness = readiness;
    derive_roadmap(&[o]).into_iter().next().unwrap()
}

#[test]
fn outcomes_come_back_flat_in_priority_order_now_then_next_then_later() {
    let cards = derive_roadmap(&[
        outcome(3, "Later thing", Some(Horizon::Later)),
        outcome(1, "Now thing", Some(Horizon::Now)),
        outcome(2, "Next thing", Some(Horizon::Next)),
    ]);
    let horizons: Vec<Horizon> = cards.iter().map(|c| c.horizon).collect();
    assert_eq!(horizons, vec![Horizon::Now, Horizon::Next, Horizon::Later]);
    assert_eq!(cards[0].name, "Now thing");
}

#[test]
fn within_a_horizon_orders_by_horizon_order_then_number() {
    let cards = derive_roadmap(&[
        with_order(outcome(10, "second", Some(Horizon::Now)), Some(2)),
        with_order(outcome(20, "first", Some(Horizon::Now)), Some(1)),
    ]);
    let names: Vec<&str> = cards.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["first", "second"]);
}

#[test]
fn an_outcome_with_no_horizon_order_sinks_below_ordered_peers_ties_by_number() {
    let cards = derive_roadmap(&[
        with_order(outcome(5, "unordered", Some(Horizon::Now)), None),
        with_order(outcome(9, "ordered", Some(Horizon::Now)), Some(1)),
    ]);
    let names: Vec<&str> = cards.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["ordered", "unordered"]);
}

#[test]
fn an_absent_horizon_is_treated_as_later() {
    let cards = derive_roadmap(&[outcome(1, "no horizon", None)]);
    assert_eq!(cards[0].horizon, Horizon::Later);
}

#[test]
fn state_all_building_iterations_complete_reads_done() {
    let card = card_for(true, Some(readiness(2, 0, true)), Horizon::Now);
    assert_eq!(card.state, OutcomeState::Done);
}

#[test]
fn state_all_ready_reads_ready() {
    let card = card_for(false, Some(readiness(1, 0, true)), Horizon::Now);
    assert_eq!(card.state, OutcomeState::Ready);
}

#[test]
fn state_with_gaps_reads_needs_attention_carrying_the_count() {
    let card = card_for(false, Some(readiness(2, 1, false)), Horizon::Now);
    assert_eq!(card.state, OutcomeState::NeedsAttention);
    assert_eq!(card.gap_count, 1);
}

#[test]
fn state_building_but_not_all_ready_reads_needs_attention() {
    let card = card_for(false, Some(readiness(1, 0, false)), Horizon::Now);
    assert_eq!(card.state, OutcomeState::NeedsAttention);
}

#[test]
fn state_no_building_iterations_or_absent_reads_planned() {
    assert_eq!(
        card_for(false, Some(readiness(0, 0, false)), Horizon::Now).state,
        OutcomeState::Planned
    );
    assert_eq!(
        card_for(false, None, Horizon::Now).state,
        OutcomeState::Planned
    );
}

#[test]
fn done_takes_precedence_over_the_readiness_rollup() {
    let card = card_for(true, Some(readiness(2, 3, false)), Horizon::Now);
    assert_eq!(card.state, OutcomeState::Done);
}

#[test]
fn tone_maps_attention_to_gap_done_and_ready_to_ready_planned_to_horizon() {
    assert_eq!(
        card_for(false, Some(readiness(2, 1, false)), Horizon::Now).tone,
        OutcomeCardTone::Gap
    );
    assert_eq!(
        card_for(false, Some(readiness(1, 0, true)), Horizon::Now).tone,
        OutcomeCardTone::Ready
    );
    assert_eq!(
        card_for(true, Some(readiness(1, 0, true)), Horizon::Now).tone,
        OutcomeCardTone::Ready
    );
    assert_eq!(
        card_for(false, None, Horizon::Now).tone,
        OutcomeCardTone::Now
    );
    assert_eq!(
        card_for(false, None, Horizon::Later).tone,
        OutcomeCardTone::Future
    );
}

#[test]
fn parse_why_extracts_benefit_metric_confidence_value_from_labelled_lines() {
    let why = parse_outcome_why(
        "## Title\n\n**Benefit:** any agent plugs in\n**Success-metric:** 0 Claude-only paths\n**Confidence:** High\n**Value:** high\n",
    );
    assert_eq!(why.benefit.as_deref(), Some("any agent plugs in"));
    assert_eq!(why.metric.as_deref(), Some("0 Claude-only paths"));
    assert_eq!(why.confidence.as_deref(), Some("High"));
    assert_eq!(why.value.as_deref(), Some("high"));
}

#[test]
fn parse_why_tolerates_plain_label_and_metric_alias() {
    let why = parse_outcome_why("Benefit: x\nMetric: y");
    assert_eq!(why.benefit.as_deref(), Some("x"));
    assert_eq!(why.metric.as_deref(), Some("y"));
}

#[test]
fn parse_why_omits_an_absent_field_distinct_from_present_but_empty() {
    let why = parse_outcome_why("**Benefit:** only this");
    assert_eq!(why.benefit.as_deref(), Some("only this"));
    assert!(why.metric.is_none());
    assert!(why.confidence.is_none());
    assert!(why.value.is_none());
}

#[test]
fn parse_why_keeps_a_present_but_empty_field_as_empty_string() {
    let why = parse_outcome_why("**Benefit:**\n**Confidence:** High");
    assert_eq!(why.benefit.as_deref(), Some(""));
    assert_eq!(why.confidence.as_deref(), Some("High"));
}

#[test]
fn the_parsed_why_is_attached_to_each_card() {
    let mut o = outcome(1, "An outcome", Some(Horizon::Now));
    o.body = "**Benefit:** plugs in\n**Confidence:** High".to_string();
    let cards = derive_roadmap(&[o]);
    assert_eq!(cards[0].why.benefit.as_deref(), Some("plugs in"));
    assert_eq!(cards[0].why.confidence.as_deref(), Some("High"));
    assert!(cards[0].why.value.is_none());
}

#[test]
fn render_labels_each_band_once_and_shows_per_outcome_state() {
    let mut now_ready = with_order(outcome(42, "Own infra", Some(Horizon::Now)), Some(1));
    now_ready.readiness = Some(readiness(1, 0, true));
    let mut now_gap = with_order(outcome(10, "Another now", Some(Horizon::Now)), Some(2));
    now_gap.readiness = Some(readiness(2, 2, false));
    let next_planned = outcome(7, "A next thing", Some(Horizon::Next));

    let text = render_roadmap(&derive_roadmap(&[now_ready, now_gap, next_planned]));
    let lines: Vec<&str> = text.lines().collect();

    assert_eq!(lines[0], "ROADMAP");
    assert!(lines[1].trim_start().starts_with("Now"));
    assert!(lines[1].contains("#42") && lines[1].ends_with("ready"));
    assert!(
        !lines[2].trim_start().starts_with("Now"),
        "the second Now outcome omits the repeated band label: {:?}",
        lines[2]
    );
    assert!(lines[2].contains("#10") && lines[2].ends_with("2 gaps"));
    assert!(lines[3].trim_start().starts_with("Next"));
    assert!(lines[3].contains("#7") && lines[3].ends_with("planned"));
}

#[test]
fn render_singularizes_a_one_gap_outcome() {
    let mut one_gap = outcome(5, "Almost", Some(Horizon::Now));
    one_gap.readiness = Some(readiness(1, 1, false));
    let line = render_roadmap(&derive_roadmap(&[one_gap]));
    let outcome_line = line.lines().nth(1).unwrap();
    assert!(outcome_line.ends_with("1 gap"), "got: {outcome_line:?}");
    assert!(!outcome_line.ends_with("gaps"));
}

#[test]
fn render_handles_no_outcomes() {
    assert_eq!(render_roadmap(&[]), "ROADMAP\n  (no outcomes yet)\n");
}
