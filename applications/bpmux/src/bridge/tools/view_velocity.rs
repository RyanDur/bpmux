use std::num::NonZeroUsize;

use serde::Serialize;

use crate::board::flow::{Position, ReworkBreakdown, desk_check_rework, sub_section_timings};
use crate::board::history::PlacementHistory;
use crate::product_management::history::SharedHistory;
use crate::product_management::iteration::Iteration;
use crate::product_management::metrics::{
    acceptance_rejection_rate, lead_time_median_hours, velocity, velocity_series,
};
use crate::product_management::story::StoryId;

const VELOCITY_WINDOW: NonZeroUsize = NonZeroUsize::MIN;

#[derive(Debug, Serialize)]
pub(in crate::bridge) struct MetricsView {
    pub velocity: Option<f64>,
    pub velocity_window_iterations: usize,
    pub iterations: Vec<IterationVelocity>,
    pub lead_time_median_hours: Option<f64>,
    pub acceptance_rejection_rate: Option<f64>,
    pub sub_section_timings: Vec<SubSectionTiming>,
    pub rework: ReworkView,
}

#[derive(Debug, Serialize)]
pub(in crate::bridge) struct IterationVelocity {
    pub iteration: u64,
    pub accepted_features: usize,
}

#[derive(Debug, Serialize)]
pub(in crate::bridge) struct SubSectionTiming {
    pub position: PositionView,
    pub median_hours: f64,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(in crate::bridge) enum PositionView {
    Stage { id: String },
    State { id: String },
}

#[derive(Debug, Serialize)]
pub(in crate::bridge) struct ReworkView {
    pub total: usize,
    pub per_story: Vec<ReworkStory>,
}

#[derive(Debug, Serialize)]
pub(in crate::bridge) struct ReworkStory {
    pub story: u64,
    pub reentries: usize,
}

impl From<Position> for PositionView {
    fn from(position: Position) -> Self {
        match position {
            Position::Placement(stage) => PositionView::Stage {
                id: stage.as_str().to_string(),
            },
            Position::Coarse(work) => PositionView::State {
                id: work.as_str().to_string(),
            },
        }
    }
}

impl From<ReworkBreakdown> for ReworkView {
    fn from(breakdown: ReworkBreakdown) -> Self {
        Self {
            total: breakdown.total,
            per_story: breakdown
                .per_story
                .into_iter()
                .map(|(story, reentries)| ReworkStory {
                    story: story.value(),
                    reentries,
                })
                .collect(),
        }
    }
}

pub(in crate::bridge) fn build_velocity_view(
    shared: &SharedHistory,
    local: &PlacementHistory,
    iterations: &[Iteration],
    is_feature: impl Fn(StoryId) -> bool,
) -> MetricsView {
    let series = velocity_series(shared, iterations, &is_feature)
        .into_iter()
        .map(|(iteration, accepted_features)| IterationVelocity {
            iteration: iteration.value(),
            accepted_features,
        })
        .collect();
    let timings = sub_section_timings(local)
        .into_iter()
        .map(|(position, median_hours)| SubSectionTiming {
            position: position.into(),
            median_hours,
        })
        .collect();
    MetricsView {
        velocity: velocity(shared, iterations, &is_feature, VELOCITY_WINDOW),
        velocity_window_iterations: VELOCITY_WINDOW.get(),
        iterations: series,
        lead_time_median_hours: lead_time_median_hours(shared),
        acceptance_rejection_rate: acceptance_rejection_rate(shared),
        sub_section_timings: timings,
        rework: desk_check_rework(local).into(),
    }
}
