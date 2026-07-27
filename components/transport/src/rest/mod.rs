mod board;
mod board_acceptance;
pub mod board_metrics;
#[cfg(all(test, feature = "test-support"))]
mod board_metrics_tests;
mod board_read;
mod board_story_detail;
pub mod broadcast_listeners;
mod capture;
mod ci;
mod classifier;
pub mod github_connection;
pub mod hydration_listeners;
mod iteration_close;
pub(crate) mod keyed_frames;
#[cfg(test)]
mod keyed_frames_tests;
mod labels;
mod lessons;
mod messages;
pub mod metrics;
mod owner_session;
pub mod presence;
mod reactions;
mod register;
mod roadmap;
mod room;
mod roster;
mod search;
pub mod shared;
pub(crate) mod story_frames;
#[cfg(test)]
mod story_frames_tests;
mod wait;
mod wake;

#[cfg(test)]
mod board_endpoint_tests;
#[cfg(test)]
mod board_tests;
#[cfg(test)]
mod register_tests;
#[cfg(test)]
mod shared_rest_helpers;
#[cfg(test)]
mod story_label_reorder;
#[cfg(test)]
mod wake_tests;

pub use board::{board_epics, board_iterations, board_outcomes, board_roadmap, board_stories};
pub use board_acceptance::{AcceptPayload, RejectPayload, post_accept_story, post_reject_story};
pub use board_read::{get_board, get_iterations, get_story};
pub use board_story_detail::{get_story_comments, get_story_timeline, post_story_comment};
pub use capture::{ElectronCaptureQuery, get_electron_capture};
pub use ci::{get_ci, get_ci_jobs, get_ci_log, post_ci_rerun};
pub use classifier::{ClassifierMisfire, post_classifier_misfire};
pub use github_connection::{
    ConnectionsHandle, delete_github_connection, get_github_connection, put_github_connection,
};
pub use iteration_close::{CloseIterationRequest, post_close_iteration};
pub use labels::{LabelMutationPayload, post_story_labels};
pub use lessons::{
    LessonsQuery, PeerLessonsQuery, SaveLessonPayload, get_lessons, get_peer_lessons, post_lesson,
};
pub use message_log::DEFAULT_MESSAGE_LIMIT;
pub use messages::{GetMessagesParams, PostMessage, get_messages, post_message};
pub use metrics::get_metrics;
pub use owner_session::{ClaimOwnerSessionRequest, post_owner_session};
pub use presence::{PresenceRequest, post_presence};
pub use reactions::{
    ReactionToggle, ReactionsChannelQuery, apply_reaction_toggle, get_reactions, post_reaction,
};
#[cfg(any(test, feature = "test-support"))]
pub use reactions::{
    build_reaction_event_text_with_excerpt, find_message_excerpt, message_scope_from_key,
    truncate_excerpt,
};
pub use register::{RegisterRequest, post_register};
pub use roadmap::get_roadmap;
pub use room::{get_terminals, post_terminal_close, post_terminal_spawn};
pub use roster::{
    RosterEntry, get_roster, get_sync_status, post_acknowledge_dropped, post_force_sync,
    roster_entries,
};
pub use search::{SearchParams, search_messages};
pub use shared::epics_message;
pub use shared::iterations_message;
pub use shared::json_headers;
pub use shared::outcomes_message;
pub use shared::roadmap_message;
pub use shared::stories_message;
pub use shared::{CommentPayload, TrackerHandle};
pub use wait::{DEFAULT_WAIT_TIMEOUT_MS, WaitParams, wait_for_event};
pub use wake::{WakeOnEventRequest, wake_on_event};
