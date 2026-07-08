#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::reactions::slice::{
    ReactionsAction, ReactionsState, aggregated_for_channel, from_store, reduce, to_store,
};
use super::reactions::{load, save};
use tempfile::tempdir;

fn toggle(
    state: ReactionsState,
    message_key: &str,
    emoji: &str,
    user: &str,
    ts: &str,
) -> ReactionsState {
    reduce(
        &state,
        &ReactionsAction::ReactionToggled {
            channel: "broadcast".to_string(),
            message_key: message_key.to_string(),
            emoji: emoji.to_string(),
            user: user.to_string(),
            ts_iso: ts.to_string(),
        },
    )
    .expect("ReactionToggled yields a new state")
}

#[test]
fn conversion_round_trips_the_aggregate_through_the_on_disk_shape() {
    let mut state = toggle(ReactionsState::default(), "k1", "👍", "owner", "t0");
    state = toggle(state, "k1", "👍", "dev", "t1");
    state = toggle(state, "k1", "🎉", "owner", "t2");

    let rehydrated = from_store(&to_store(&state));

    assert_eq!(
        aggregated_for_channel(&rehydrated, "broadcast"),
        aggregated_for_channel(&state, "broadcast"),
    );
}

#[test]
fn save_then_load_is_lossless_including_the_reacted_at_timestamp() {
    let dir = tempdir().unwrap();
    let state = toggle(
        ReactionsState::default(),
        "k1",
        "👍",
        "owner",
        "2026-07-08T00:00:00Z",
    );

    save(dir.path(), &to_store(&state)).unwrap();
    let loaded = load(dir.path());

    assert_eq!(
        loaded.channels.get("broadcast").map(|list| list.as_slice()),
        to_store(&state)
            .channels
            .get("broadcast")
            .map(|list| list.as_slice()),
    );
    assert_eq!(
        loaded.channels["broadcast"][0].reacted_at,
        "2026-07-08T00:00:00Z"
    );

    let rehydrated = from_store(&loaded);
    assert_eq!(
        aggregated_for_channel(&rehydrated, "broadcast"),
        aggregated_for_channel(&state, "broadcast"),
    );
}
