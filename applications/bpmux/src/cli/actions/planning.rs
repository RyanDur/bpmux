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
    #[command(
        about = "Remove a story from an iteration (the only INTENTIONAL vanish-path; the story is otherwise untouched). --reason required (destructive scope-cut, the audit-trail). PM-only; owner bypasses."
    )]
    RemoveMember {
        #[arg(help = "Iteration entity issue number")]
        iteration: u64,
        #[arg(help = "Member story issue number to remove")]
        story: u64,
        #[arg(long, help = "Why it's being removed (required — the audit-trail)")]
        reason: String,
    },
    #[command(about = "Reorder a story to a 1-indexed position within an iteration. PM-only.")]
    MoveMember {
        #[arg(help = "Iteration entity issue number")]
        iteration: u64,
        #[arg(help = "Member story issue number to reorder")]
        story: u64,
        #[arg(long, help = "New 1-indexed position within the iteration")]
        to: u32,
    },
    #[command(about = "Edit an iteration's theme (its title). PM-only; owner bypasses.")]
    Edit {
        #[arg(help = "Iteration entity issue number")]
        number: u64,
        #[arg(long, help = "New theme name")]
        theme: String,
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
    #[command(
        about = "Edit an outcome's title (the headline why) and/or body (supplementary why-fields) — the roadmap-semantic verb, NOT the generic `bpmux edit <outcome#>` which leaks outcome=story. At least one of --title / --body. PM-only; owner bypasses.",
        group = clap::ArgGroup::new("outcome_edit_change").required(true).multiple(true).args(["title", "body"])
    )]
    Edit {
        #[arg(help = "Outcome entity issue number")]
        number: u64,
        #[arg(
            long,
            help = "New title — the measurable behaviour-change (the headline why, surfaced in bpmux roadmap)"
        )]
        title: Option<String>,
        #[arg(
            long,
            help = "New body — supplementary why-fields (the disclosure-detail, surfaced via bpmux story)"
        )]
        body: Option<String>,
    },
    #[command(
        about = "Re-prioritize an outcome across Now/Next/Later — move its horizon band and/or reorder it within a band (the roadmap's reorganize, in roadmap-language not story-internals). At least one of --to / --position. PM-only; owner bypasses.",
        group = clap::ArgGroup::new("reprioritize_target").required(true).multiple(true).args(["to", "position"])
    )]
    Reprioritize {
        #[arg(help = "Outcome entity issue number")]
        number: u64,
        #[arg(
            long,
            help = "Target band: now | next | later (append to the band by default)"
        )]
        to: Option<String>,
        #[arg(
            long,
            help = "1-indexed position within the band (reorder; clamps to the band end on overshoot)"
        )]
        position: Option<u32>,
    },
    #[command(
        about = "Retire an outcome (domain soft-delete) so it no longer appears in bpmux roadmap — the roadmap-semantic verb, NOT the generic `bpmux close <outcome#>`. --reason required (destructive; the reason is the stream audit-trail). PM-only; owner bypasses."
    )]
    Retire {
        #[arg(help = "Outcome entity issue number")]
        number: u64,
        #[arg(
            long,
            help = "Why it's being retired (required — the audit-trail in the stream)"
        )]
        reason: String,
    },
}
