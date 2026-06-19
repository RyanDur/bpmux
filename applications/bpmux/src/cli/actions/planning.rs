use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum EpicAction {
    #[command(about = "Group a child story under an Epic")]
    SetParent {
        #[arg(help = "Child story number")]
        number: u64,
        #[arg(long, help = "Epic issue number that owns the child")]
        epic: u64,
    },
    #[command(about = "Ungroup a child story from its current Epic")]
    UnsetParent {
        #[arg(help = "Child story number")]
        number: u64,
    },
    #[command(about = "List the child stories under an Epic")]
    Children {
        #[arg(help = "Epic issue number")]
        epic: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum IterationAction {
    #[command(about = "Create an iteration with a theme")]
    Create {
        #[arg(long, help = "Iteration theme name")]
        theme: String,
        #[arg(long, help = "Insert after this iteration number (optional)")]
        position_after: Option<u64>,
    },
    #[command(about = "Designate the current iteration by number")]
    Designate {
        #[arg(help = "Iteration number")]
        number: u64,
    },
    #[command(about = "Add a member story to an iteration at a 1-indexed position")]
    Add {
        #[arg(long, help = "Iteration entity issue number")]
        iteration: u64,
        #[arg(long, help = "Member story issue number")]
        story: u64,
        #[arg(long, help = "1-indexed position within the iteration")]
        position: u64,
    },
    #[command(about = "Close an iteration (terminal; the ready queue must be empty)")]
    Close {
        #[arg(long, help = "Iteration entity issue number")]
        iteration: u64,
    },
    #[command(
        about = "Close the pre-pickup review for an iteration (iteration-reviewers only; derives the review-readiness check)"
    )]
    CloseReview {
        #[arg(help = "Iteration entity issue number")]
        number: u64,
    },
    #[command(
        about = "Record dev-OK-at-IPM for an iteration (iteration-builders only; derives the dev-OK readiness check)"
    )]
    DevOk {
        #[arg(help = "Iteration entity issue number")]
        number: u64,
    },
}

#[derive(Subcommand)]
pub(crate) enum OutcomeAction {
    #[command(about = "Create a first-class roadmap outcome (the why, horizon-sequenced)")]
    Create {
        #[arg(long, help = "The measurable behaviour-change the outcome pursues")]
        title: String,
        #[arg(long, help = "Horizon: now | next | later (sequence, not calendar)")]
        horizon: String,
        #[arg(long, help = "Outcome body markdown (why-fields); optional")]
        body: Option<String>,
    },
    #[command(
        about = "Map an iteration to an outcome at a position (the iterations that build it, in order)"
    )]
    Add {
        #[arg(long, help = "Outcome entity issue number")]
        outcome: u64,
        #[arg(long, help = "Iteration entity issue number that builds the outcome")]
        iteration: u64,
        #[arg(long, help = "1-indexed position in the outcome's iteration-sequence")]
        position: u64,
    },
}
