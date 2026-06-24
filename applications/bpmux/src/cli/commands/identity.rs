use clap::Subcommand;

use crate::cli::actions::{
    BookmarkAction, BriefingAction, CiAction, LessonAction, PrincipleAction, ValueAction,
};
use crate::cli::bookmark::{run_bookmark_get, run_bookmark_save};
use crate::cli::briefing::{run_briefing_get, run_briefing_save};
use crate::cli::ci::{run_ci_jobs, run_ci_log, run_ci_rerun, run_ci_state};
use crate::cli::lesson::{run_lesson_list, run_lesson_list_peer, run_lesson_save};
use crate::cli::principle::{
    run_identity_get, run_principle_add, run_principle_remove, run_principle_update, run_value_add,
};

#[derive(Subcommand)]
pub(crate) enum IdentityCommands {
    #[command(
        about = "Lesson operations off Claude's MCP — save/list lessons via the same /api/lessons + /api/peer-lessons endpoints the bridge uses (ungated; author_role from BPMUX_ROLE)."
    )]
    Lesson {
        #[command(subcommand)]
        action: LessonAction,
    },
    #[command(
        about = "CI operations off Claude's MCP — state/log/jobs/rerun via the same Bearer-auth /api/ci endpoints the bridge uses (no role-guard, mirroring the MCP ci tools)."
    )]
    Ci {
        #[command(subcommand)]
        action: CiAction,
    },
    #[command(
        about = "Principle operations off Claude's MCP — add/update/remove principles in this role's identity (filesystem-direct, ungated; same crate::memory::identity store the bridge uses)."
    )]
    Principle {
        #[command(subcommand)]
        action: PrincipleAction,
    },
    #[command(
        about = "Value operations off Claude's MCP — add an identity value to this role's identity (filesystem-direct, ungated)."
    )]
    Value {
        #[command(subcommand)]
        action: ValueAction,
    },
    #[command(
        about = "Print this role's identity (values + principles) off Claude's MCP, from the same crate::memory::identity store the bridge uses."
    )]
    Identity,
    #[command(
        about = "Bookmark operations off Claude's MCP — save/get a session-continuity bookmark for this role (filesystem-direct in the current session dir; ungated; same crate::session::bookmark store the bridge uses)."
    )]
    Bookmark {
        #[command(subcommand)]
        action: BookmarkAction,
    },
    #[command(
        about = "Briefing operations off Claude's MCP — save/get this role's briefing (filesystem-direct under BPMUX_BRIEFINGS_DIR; ungated; same crate::session::briefing store the bridge uses)."
    )]
    Briefing {
        #[command(subcommand)]
        action: BriefingAction,
    },
}

pub(crate) fn dispatch_identity(command: &IdentityCommands) -> anyhow::Result<()> {
    match command {
        IdentityCommands::Lesson { action } => match action {
            LessonAction::Save {
                topic,
                lesson,
                evidence,
            } => run_lesson_save(topic, lesson, evidence),
            LessonAction::List { topic, limit } => run_lesson_list(topic, *limit),
            LessonAction::ListPeer { peer } => run_lesson_list_peer(peer),
        },
        IdentityCommands::Ci { action } => match action {
            CiAction::State => run_ci_state(),
            CiAction::Log { run } => run_ci_log(run),
            CiAction::Jobs { run } => run_ci_jobs(run),
            CiAction::Rerun { run } => run_ci_rerun(run),
        },
        IdentityCommands::Principle { action } => match action {
            PrincipleAction::Add { value, principle } => run_principle_add(value, principle),
            PrincipleAction::Update { value, old, new } => run_principle_update(value, old, new),
            PrincipleAction::Remove { value, principle } => run_principle_remove(value, principle),
        },
        IdentityCommands::Value { action } => match action {
            ValueAction::Add { name, description } => run_value_add(name, description),
        },
        IdentityCommands::Identity => run_identity_get(),
        IdentityCommands::Bookmark { action } => match action {
            BookmarkAction::Save {
                current_task,
                next_step,
                notes,
                decisions,
                open_questions,
            } => run_bookmark_save(current_task, next_step, notes, decisions, open_questions),
            BookmarkAction::Get { role } => run_bookmark_get(role),
        },
        IdentityCommands::Briefing { action } => match action {
            BriefingAction::Save { content } => run_briefing_save(content),
            BriefingAction::Get { role } => run_briefing_get(role),
        },
    }
}
