mod actions;
mod bookmark;
mod briefing;
mod ci;
mod commands;
mod help;
mod http;
mod lesson;
mod messaging;
#[cfg(test)]
mod parity_audit;
mod principle;
mod read;
#[cfg(test)]
mod read_tests;
mod write;
mod write_iteration;
mod write_query;
mod write_story;
pub(crate) use actions::{
    BookmarkAction, BriefingAction, CiAction, EpicAction, IterationAction, LessonAction,
    OutcomeAction, PrincipleAction, RoomAction, ValueAction,
};
pub(crate) use bookmark::{run_bookmark_get, run_bookmark_save};
pub(crate) use briefing::{run_briefing_get, run_briefing_save};
pub(crate) use ci::{run_ci_jobs, run_ci_log, run_ci_rerun, run_ci_state};
pub(crate) use commands::Command;
pub(crate) use help::run_help;
pub(crate) use lesson::{run_lesson_list, run_lesson_list_peer, run_lesson_save};
pub(crate) use messaging::{
    emit_recall_activity, run_emit_precommit_passed, run_read, run_register, run_send,
    run_wait_for_event,
};
pub(crate) use principle::{
    run_identity_get, run_principle_add, run_principle_remove, run_principle_update, run_value_add,
};
pub(crate) use read::{
    run_board, run_capture, run_ready, run_roadmap, run_search, run_story, run_sync_status,
};
pub(crate) use write::run_signal;
pub(crate) use write_iteration::{
    run_epic_children, run_epic_set_parent, run_epic_unset_parent, run_iteration_add,
    run_iteration_close, run_iteration_close_review, run_iteration_create, run_iteration_designate,
    run_iteration_dev_ok, run_outcome_add, run_outcome_create,
};
pub(crate) use write_query::{
    run_backlog, run_comments, run_commit_diff, run_story_commits, run_timeline, run_velocity,
    run_verdicts,
};
pub(crate) use write_story::{
    run_accept, run_close, run_comment, run_create, run_deliver, run_edit, run_fail, run_label,
    run_move, run_pass, run_reject, run_start, run_verdict,
};
pub(crate) fn dispatch(command: &Command) -> Option<anyhow::Result<()>> {
    match command {
        Command::EmitPrecommitPassed { sha, role } => Some(run_emit_precommit_passed(sha, role)),
        Command::WaitForEvent { after, timeout_ms } => Some(run_wait_for_event(after, *timeout_ms)),
        Command::Send {
            text,
            to,
            addressee,
        } => Some(run_send(text, to, addressee)),
        Command::Register => Some(run_register()),
        Command::Read { limit, full } => Some(run_read(*limit, *full)),
        Command::Help { json } => Some(run_help(*json)),
        Command::Ready { json, full } => Some(run_ready(*json, *full)),
        Command::Story { number, json, full } => Some(run_story(*number, *json, *full)),
        Command::Board {
            state,
            limit,
            json,
            full,
        } => Some(run_board(state, *limit, *json, *full)),
        Command::Capture { window } => Some(run_capture(*window)),
        Command::Pass { number, acceptor } => Some(run_pass(*number, acceptor)),
        Command::Start {
            number,
            paraphrase,
            falsifier,
        } => Some(run_start(*number, paraphrase, falsifier)),
        Command::Deliver {
            number,
            observation,
        } => Some(run_deliver(*number, observation)),
        Command::Fail { number, reason } => Some(run_fail(*number, reason)),
        Command::Reject { number, reason } => Some(run_reject(*number, reason)),
        Command::Signal { state } => Some(run_signal(state)),
        Command::Verdict {
            number,
            verdict,
            grounding,
        } => Some(run_verdict(*number, verdict, grounding)),
        Command::Accept {
            number,
            would_file_bug,
            force,
            bypass_reason,
        } => Some(run_accept(*number, *would_file_bug, *force, bypass_reason)),
        Command::Move {
            number,
            target_state,
            rationale,
            position,
        } => Some(run_move(*number, target_state, rationale, *position)),
        Command::Label {
            number,
            label,
            operation,
        } => Some(run_label(*number, label, operation)),
        Command::Create {
            title,
            user_job,
            observation_falsifier,
            body,
            parent_epic,
            acceptance_test_path,
        } => Some(run_create(
            title,
            user_job,
            observation_falsifier,
            body,
            *parent_epic,
            acceptance_test_path,
        )),
        Command::Edit {
            number,
            title,
            body,
        } => Some(run_edit(*number, title, body)),
        Command::Comment { number, body } => Some(run_comment(*number, body)),
        Command::Close {
            number,
            reason,
            disposition,
        } => Some(run_close(*number, reason, disposition)),
        Command::Iteration { action } => Some(match action {
            IterationAction::Create {
                theme,
                position_after,
            } => run_iteration_create(theme, *position_after),
            IterationAction::Designate { number } => run_iteration_designate(*number),
            IterationAction::Add {
                iteration,
                story,
                position,
            } => run_iteration_add(*iteration, *story, *position),
            IterationAction::Close { iteration } => run_iteration_close(*iteration),
            IterationAction::CloseReview { number } => run_iteration_close_review(*number),
            IterationAction::DevOk { number } => run_iteration_dev_ok(*number),
        }),
        Command::Outcome { action } => Some(match action {
            OutcomeAction::Create {
                title,
                horizon,
                body,
            } => run_outcome_create(title, horizon, body.as_deref()),
            OutcomeAction::Add {
                outcome,
                iteration,
                position,
            } => run_outcome_add(*outcome, *iteration, *position),
        }),
        Command::Epic { action } => Some(match action {
            EpicAction::SetParent { number, epic } => run_epic_set_parent(*number, *epic),
            EpicAction::UnsetParent { number } => run_epic_unset_parent(*number),
            EpicAction::Children { epic } => run_epic_children(*epic),
        }),
        Command::Backlog => Some(run_backlog()),
        Command::Comments { story } => Some(run_comments(*story)),
        Command::Timeline { story } => Some(run_timeline(*story)),
        Command::Verdicts { story } => Some(run_verdicts(*story)),
        Command::Velocity => Some(run_velocity()),
        Command::Commits { story } => Some(run_story_commits(*story)),
        Command::Diff { sha, max_bytes } => Some(run_commit_diff(sha, *max_bytes)),
        Command::Roadmap => Some(run_roadmap()),
        Command::SyncStatus => Some(run_sync_status()),
        Command::Search { query, sender } => Some(run_search(query, sender)),
        Command::Lesson { action } => Some(match action {
            LessonAction::Save {
                topic,
                lesson,
                evidence,
            } => run_lesson_save(topic, lesson, evidence),
            LessonAction::List { topic, limit } => run_lesson_list(topic, *limit),
            LessonAction::ListPeer { peer } => run_lesson_list_peer(peer),
        }),
        Command::Ci { action } => Some(match action {
            CiAction::State => run_ci_state(),
            CiAction::Log { run } => run_ci_log(run),
            CiAction::Jobs { run } => run_ci_jobs(run),
            CiAction::Rerun { run } => run_ci_rerun(run),
        }),
        Command::Principle { action } => Some(match action {
            PrincipleAction::Add { value, principle } => run_principle_add(value, principle),
            PrincipleAction::Update { value, old, new } => run_principle_update(value, old, new),
            PrincipleAction::Remove { value, principle } => run_principle_remove(value, principle),
        }),
        Command::Value { action } => Some(match action {
            ValueAction::Add { name, description } => run_value_add(name, description),
        }),
        Command::Identity => Some(run_identity_get()),
        Command::Bookmark { action } => Some(match action {
            BookmarkAction::Save {
                current_task,
                next_step,
                notes,
                decisions,
                open_questions,
            } => run_bookmark_save(current_task, next_step, notes, decisions, open_questions),
            BookmarkAction::Get { role } => run_bookmark_get(role),
        }),
        Command::Briefing { action } => Some(match action {
            BriefingAction::Save { content } => run_briefing_save(content),
            BriefingAction::Get { role } => run_briefing_get(role),
        }),
        Command::Bridge { .. }
        | Command::Recall { .. }
        | Command::Room { .. }
        | Command::Server => None,
    }
}
