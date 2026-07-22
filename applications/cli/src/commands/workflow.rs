use clap::Subcommand;

use crate::write::run_signal;
use crate::write_story::{
    run_accept, run_deliver, run_fail, run_fail_stage, run_pass, run_pass_stage, run_reject,
    run_start, run_verdict,
};

#[derive(Subcommand)]
pub enum WorkflowCommands {
    #[command(
        about = "Pass a story's desk-check as this agent (ambient role via BPMUX_ROLE), off Claude's MCP — fires the same gated /api/rpc/pass_desk_check path the bridge uses, so the role-guard, verdict-evidence, and CI-green gates apply identically. --acceptor voices the acceptor-class at the gate."
    )]
    Pass {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Acceptor class voiced at the gate: team | owner | dev")]
        acceptor: String,
    },
    #[command(
        about = "Start (pick up) a ready story as this agent off Claude's MCP — fires the same gated /api/rpc/start_story path the bridge uses (dev-only pickup role-guard; owner bypasses via the #1548 superset)."
    )]
    Start {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Your restatement of the user-job at pickup")]
        paraphrase: String,
        #[arg(long, help = "Your reframing of the observation-falsifier")]
        falsifier: String,
    },
    #[command(
        about = "Mark a story delivered as this agent off Claude's MCP — fires the same gated /api/rpc/deliver_story path the bridge uses (dev-only; owner bypasses)."
    )]
    Deliver {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What you drove end-to-end (the desk-check diff-point)")]
        observation: String,
    },
    #[command(
        about = "Signal your own collaboration state off Claude's MCP — pushes busy|idle|help to the gated /api/presence (the same endpoint the agent-hooks use), setting your roster presence + collaboration-dot. needs-help is reachable here."
    )]
    Signal {
        #[arg(help = "Your collaboration state: busy | idle | help")]
        state: String,
    },
    #[command(
        about = "Fail a story's desk-check as this agent off Claude's MCP — fires the same gated /api/rpc/fail_desk_check path the bridge uses (pm/designer role-guard; owner bypasses)."
    )]
    Fail {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What failed — surfaced to dev as the workflow signal")]
        reason: String,
    },
    #[command(
        about = "Reject a delivered story as this agent off Claude's MCP — fires the same gated /api/rpc/reject_story path the bridge uses (pm role-guard; owner bypasses)."
    )]
    Reject {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Rejection reason — recorded as a comment on the story")]
        reason: Option<String>,
    },
    #[command(
        name = "pass-stage",
        about = "Pass the judgment of a story's CURRENT middle stage off Claude's MCP — the config-generic progression verb; fires the same gated /api/rpc/pass_stage path the bridge uses. The workflow config decides the next stage and who may judge (stage judges; anyone when unset; owner always)."
    )]
    PassStage {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What you verified at this stage in the running app")]
        observation: String,
    },
    #[command(
        name = "fail-stage",
        about = "Fail the judgment of a story's CURRENT middle stage off Claude's MCP — the config-generic send-back verb; fires the same gated /api/rpc/fail_stage path the bridge uses. The config decides where the story lands (fail_to, else one stage back; a first-stage failure reworks in place)."
    )]
    FailStage {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "What failed, cited concretely (min 16 chars)")]
        reason: String,
    },
    #[command(
        about = "Cast a substrate-grounded verdict on a story off Claude's MCP — fires the same gated /api/rpc/submit_verdict handler the bridge uses (dev/designer-only guard; the owner is not an evidence role and is refused, identical to MCP)."
    )]
    Verdict {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(long, help = "Verdict outcome, e.g. verified | rejected")]
        verdict: String,
        #[arg(
            long,
            help = "Required source-grounded cite for the verdict (non-empty)"
        )]
        grounding: String,
    },
    #[command(
        about = "Accept a delivered story off Claude's MCP — fires the same gated /api/rpc/accept_story handler the bridge uses (role-guard by gate-class + evidence + CI + bug-question), NOT the gate-skipping Electron-accept REST. The owner bypasses the role-guard via the #1548 superset."
    )]
    Accept {
        #[arg(help = "Story number")]
        number: u64,
        #[arg(
            long,
            help = "Bug-question: would you file a bug if you accepted this as-is? (blocks unless --force + --bypass-reason)"
        )]
        would_file_bug: bool,
        #[arg(
            long,
            help = "Override the bug-question block (requires --bypass-reason)"
        )]
        force: bool,
        #[arg(
            long,
            help = "Justification recorded in the gate-comment when --force overrides the bug-question"
        )]
        bypass_reason: Option<String>,
    },
}

pub fn dispatch_workflow(command: &WorkflowCommands) -> anyhow::Result<()> {
    match command {
        WorkflowCommands::Pass { number, acceptor } => run_pass(*number, acceptor),
        WorkflowCommands::Start {
            number,
            paraphrase,
            falsifier,
        } => run_start(*number, paraphrase, falsifier),
        WorkflowCommands::Deliver {
            number,
            observation,
        } => run_deliver(*number, observation),
        WorkflowCommands::Fail { number, reason } => run_fail(*number, reason),
        WorkflowCommands::PassStage {
            number,
            observation,
        } => run_pass_stage(*number, observation),
        WorkflowCommands::FailStage { number, reason } => run_fail_stage(*number, reason),
        WorkflowCommands::Reject { number, reason } => run_reject(*number, reason),
        WorkflowCommands::Signal { state } => run_signal(state),
        WorkflowCommands::Verdict {
            number,
            verdict,
            grounding,
        } => run_verdict(*number, verdict, grounding),
        WorkflowCommands::Accept {
            number,
            would_file_bug,
            force,
            bypass_reason,
        } => run_accept(*number, *would_file_bug, *force, bypass_reason),
    }
}
