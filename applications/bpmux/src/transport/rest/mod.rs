//! HTTP REST surface for the bpmux server — organized by feature-area
//! submodule. Every route registered at `server.rs::build_router` is
//! served by one of the handlers re-exported here; the `mod.rs` itself
//! holds only module declarations + `pub use` re-exports so the
//! `crate::transport::rest::*` paths consumers depend on stay stable.

mod board;
mod capture;
mod ci;
mod classifier;
mod iteration_close;
mod lessons;
mod messages;
mod metrics;
mod pins;
mod reactions;
mod roster;
mod search;
mod shared;

pub use board::{
    AcceptPayload, LabelMutationPayload, RejectPayload, get_board, get_story, get_story_comments,
    get_story_timeline, post_accept_story, post_reject_story, post_story_comment,
    post_story_labels,
};
pub use capture::{ElectronCaptureQuery, get_electron_capture, get_screen};
pub use ci::{get_ci, get_ci_log, post_ci_rerun};
pub use classifier::{ClassifierMisfire, post_classifier_misfire};
pub use iteration_close::{CloseIterationRequest, post_close_iteration};
#[allow(unused_imports)]
pub(crate) use iteration_close::{
    IterationCloseBlocker, collect_ready_queue_blockers, promote_members_then_close_iteration,
};
pub use lessons::{
    LessonsQuery, PeerLessonsQuery, SaveLessonPayload, get_lessons, get_peer_lessons, post_lesson,
};
pub use messages::{
    DEFAULT_MESSAGE_LIMIT, GetMessagesParams, PostMessage, get_messages, post_message,
};
pub use metrics::get_metrics;
pub use pins::{PinCreate, PinDelete, PinsChannelQuery, delete_pin, get_pins, post_pin};
#[allow(unused_imports)]
pub(crate) use reactions::{
    EXCERPT_MAX_LEN, build_reaction_event_text_with_excerpt, find_message_excerpt, truncate_excerpt,
};
pub use reactions::{ReactionToggle, ReactionsChannelQuery, get_reactions, post_reaction};
pub use roster::{
    RosterEntry, get_roster, get_sync_status, post_acknowledge_dropped, post_force_sync,
};
pub use search::{SearchParams, search_messages};
pub(crate) use shared::json_headers;
pub use shared::{CommentPayload, TrackerHandle};
