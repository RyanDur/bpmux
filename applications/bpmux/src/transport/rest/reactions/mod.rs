mod excerpt;
mod handlers;

use serde::Deserialize;

#[cfg(test)]
pub(crate) use excerpt::{
    build_reaction_event_text_with_excerpt, find_message_excerpt, message_scope_from_key,
    truncate_excerpt,
};
pub use handlers::{apply_reaction_toggle, get_reactions, post_reaction};

#[derive(Deserialize)]
pub struct ReactionsChannelQuery {
    pub channel: String,
}

#[derive(Deserialize)]
pub struct ReactionToggle {
    pub channel: String,
    pub message_key: String,
    pub emoji: String,
    pub user: String,
}
