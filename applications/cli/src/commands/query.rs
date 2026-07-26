use clap::Subcommand;

use crate::read::{run_search, run_sync_status};
use crate::roadmap::run_roadmap;
use crate::write_query::{
    run_backlog, run_comments, run_commit_diff, run_story_commits, run_timeline, run_velocity,
    run_verdicts,
};

#[derive(Subcommand)]
pub enum QueryCommands {
    #[command(
        about = "Show the backlog summary off Claude's MCP — fires the same gated /api/rpc/view_backlog read the bridge uses."
    )]
    Backlog,
    #[command(
        about = "Show a story's comments off Claude's MCP — fires the same gated /api/rpc/view_comments read the bridge uses."
    )]
    Comments {
        #[arg(help = "Story number")]
        story: u64,
    },
    #[command(
        about = "Show a story's timeline off Claude's MCP — fires the same gated /api/rpc/view_story_timeline read the bridge uses."
    )]
    Timeline {
        #[arg(help = "Story number")]
        story: u64,
    },
    #[command(
        about = "Show a story's verdicts off Claude's MCP — fires the same gated /api/rpc/view_verdicts read the bridge uses."
    )]
    Verdicts {
        #[arg(help = "Story number")]
        story: u64,
    },
    #[command(
        about = "Show feature-velocity metrics off Claude's MCP — fires the same gated /api/rpc/view_velocity read the bridge uses."
    )]
    Velocity,
    #[command(
        about = "List the commits that reference a story off Claude's MCP — fires the same gated /api/rpc/view_story_commits read the bridge uses."
    )]
    Commits {
        #[arg(help = "Story number")]
        story: u64,
    },
    #[command(
        about = "Show a commit's diff off Claude's MCP — fires the same gated /api/rpc/view_commit_diff read the bridge uses."
    )]
    Diff {
        #[arg(help = "Commit SHA or ref")]
        sha: String,
        #[arg(long, help = "Truncate the diff to this many bytes")]
        max_bytes: Option<u64>,
    },
    #[command(
        about = "Print the substrate-derived roadmap-view off Claude's MCP — the shared roadmap (outcomes by Now/Next/Later priority with done/ready/needs-attention state), parity with the owner's GUI. --outcome drills into one outcome's iterations."
    )]
    Roadmap {
        #[arg(long, help = "Drill into one outcome's iterations by number")]
        outcome: Option<u64>,
        #[arg(
            long,
            help = "Filter to a state: done | ready | needs-attention | planned"
        )]
        state: Option<String>,
        #[arg(long, help = "Sort key: priority (default) | gap-count | state | name")]
        order: Option<String>,
        #[arg(
            long,
            help = "Cap to the top-N outcomes (the elided count is labelled, never silently dropped)"
        )]
        limit: Option<usize>,
    },
    #[command(
        about = "Show the canonical-store sync status off Claude's MCP — the same /api/sync-status read the bridge uses."
    )]
    SyncStatus,
    #[command(
        about = "Search the message log off Claude's MCP — the same /api/search read the bridge uses."
    )]
    Search {
        #[arg(help = "Search query")]
        query: String,
        #[arg(long, help = "Filter to a sender role")]
        sender: Option<String>,
        #[arg(long, help = "Only messages addressed to this role or user id")]
        addressee: Option<String>,
    },
}

pub fn dispatch_query(command: &QueryCommands) -> anyhow::Result<()> {
    match command {
        QueryCommands::Backlog => run_backlog(),
        QueryCommands::Comments { story } => run_comments(*story),
        QueryCommands::Timeline { story } => run_timeline(*story),
        QueryCommands::Verdicts { story } => run_verdicts(*story),
        QueryCommands::Velocity => run_velocity(),
        QueryCommands::Commits { story } => run_story_commits(*story),
        QueryCommands::Diff { sha, max_bytes } => run_commit_diff(sha, *max_bytes),
        QueryCommands::Roadmap {
            outcome,
            state,
            order,
            limit,
        } => run_roadmap(*outcome, state.clone(), order.clone(), *limit),
        QueryCommands::SyncStatus => run_sync_status(),
        QueryCommands::Search {
            query,
            sender,
            addressee,
        } => run_search(query, sender, addressee),
    }
}
