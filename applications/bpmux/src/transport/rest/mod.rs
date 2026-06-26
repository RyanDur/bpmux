mod board;
mod board_acceptance;
mod board_read;
mod board_story_detail;
mod capture;
mod ci;
mod classifier;
pub(crate) mod iteration_close;
mod labels;
mod lessons;
mod messages;
mod metrics;
mod presence;
mod reactions;
mod register;
mod roadmap;
mod room;
mod roster;
mod search;
pub(crate) mod shared;
mod wait;
mod wake;

#[cfg(test)]
mod board_endpoint;
#[cfg(test)]
mod presence_endpoint;
#[cfg(test)]
mod shared_rest_helpers;
#[cfg(test)]
mod story_label_reorder;

pub(crate) use board::ordered_board_stories;
pub use board_acceptance::{AcceptPayload, RejectPayload, post_accept_story, post_reject_story};
pub use board_read::{get_board, get_story};
pub use board_story_detail::{get_story_comments, get_story_timeline, post_story_comment};
pub use capture::{ElectronCaptureQuery, get_electron_capture};
pub use ci::{get_ci, get_ci_jobs, get_ci_log, post_ci_rerun};
pub use classifier::{ClassifierMisfire, post_classifier_misfire};
pub use iteration_close::{CloseIterationRequest, post_close_iteration};
#[cfg(test)]
pub(crate) use iteration_close::{close_iteration_canonical, collect_ready_queue_blockers};
pub use labels::{LabelMutationPayload, post_story_labels};
pub use lessons::{
    LessonsQuery, PeerLessonsQuery, SaveLessonPayload, get_lessons, get_peer_lessons, post_lesson,
};
pub use messages::{
    DEFAULT_MESSAGE_LIMIT, GetMessagesParams, PostMessage, get_messages, post_message,
};
pub use metrics::get_metrics;
pub use presence::{PresenceRequest, post_presence};
pub use reactions::{
    ReactionToggle, ReactionsChannelQuery, apply_reaction_toggle, get_reactions, post_reaction,
};
#[cfg(test)]
pub(crate) use reactions::{
    build_reaction_event_text_with_excerpt, find_message_excerpt, message_scope_from_key,
    truncate_excerpt,
};
pub use register::{RegisterRequest, post_register};
pub use roadmap::get_roadmap;
pub use room::{get_terminals, post_terminal_close, post_terminal_spawn};
pub use roster::{
    RosterEntry, get_roster, get_sync_status, post_acknowledge_dropped, post_force_sync,
};
pub use search::{SearchParams, search_messages};
pub(crate) use shared::board_snapshot_message;
pub(crate) use shared::json_headers;
pub use shared::{CommentPayload, TrackerHandle};
pub use wait::{DEFAULT_WAIT_TIMEOUT_MS, WaitParams, wait_for_event};
pub use wake::{WakeOnEventRequest, wake_on_event};
