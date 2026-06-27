#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use uuid::Uuid;

use super::StoryKey;

#[test]
fn pending_and_saved_keys_coexist_in_one_map_without_collision() {
    let pending = StoryKey::Pending(Uuid::new_v4());
    let saved = StoryKey::Saved(7);

    let mut map: HashMap<StoryKey, &str> = HashMap::new();
    map.insert(pending, "optimistic");
    map.insert(saved, "from-github");

    assert_eq!(
        map.len(),
        2,
        "a pending key never collides with a saved key"
    );
    assert_eq!(*map.get(&pending).unwrap(), "optimistic");
    assert_eq!(*map.get(&saved).unwrap(), "from-github");
}

#[test]
fn saved_keys_with_the_same_number_are_the_same_handle() {
    let mut map: HashMap<StoryKey, u32> = HashMap::new();
    map.insert(StoryKey::Saved(42), 1);
    map.insert(StoryKey::Saved(42), 2);

    assert_eq!(map.len(), 1, "equal numbers address the same story");
    assert_eq!(*map.get(&StoryKey::Saved(42)).unwrap(), 2);
}

#[test]
fn distinct_pending_handles_address_distinct_stories() {
    let first = StoryKey::Pending(Uuid::new_v4());
    let second = StoryKey::Pending(Uuid::new_v4());

    assert_ne!(
        first, second,
        "each optimistic create mints its own pending handle"
    );
}

#[test]
fn saved_key_displays_as_hash_number_and_unwraps_to_its_number() {
    let key = StoryKey::Saved(42);

    assert_eq!(
        key.to_string(),
        "#42",
        "a saved story shows its GitHub number"
    );
    assert_eq!(
        key.saved_number(),
        Some(42),
        "the edge unwraps a saved key to its raw GitHub number"
    );
}

#[test]
fn pending_key_displays_as_unsaved_and_has_no_saved_number() {
    let key = StoryKey::Pending(Uuid::new_v4());

    assert_eq!(
        key.to_string(),
        "(unsaved)",
        "a pending story has no number to show"
    );
    assert_eq!(
        key.saved_number(),
        None,
        "a pending key carries no GitHub number for the edge to use"
    );
}
