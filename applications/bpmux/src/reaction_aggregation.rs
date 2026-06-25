#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::reactions::*;
use tempfile::tempdir;

#[test]
fn toggle_adds_then_removes_for_same_user_emoji() {
    let mut store = ReactionsStore::default();
    assert!(toggle_reaction(
        &mut store,
        "broadcast",
        "k1",
        "👍",
        "owner",
        "t0"
    ));
    assert!(!toggle_reaction(
        &mut store,
        "broadcast",
        "k1",
        "👍",
        "owner",
        "t1"
    ));
    assert_eq!(store.channels.get("broadcast"), None);
}

#[test]
fn different_users_aggregate_under_same_emoji() {
    let mut store = ReactionsStore::default();
    toggle_reaction(&mut store, "broadcast", "k1", "👍", "owner", "t0");
    toggle_reaction(&mut store, "broadcast", "k1", "👍", "dev", "t1");
    let agg = aggregate_for_channel(&store, "broadcast");
    assert_eq!(agg.len(), 1);
    assert_eq!(agg[0].count, 2);
    assert!(agg[0].users.contains(&"owner".to_string()));
    assert!(agg[0].users.contains(&"dev".to_string()));
}

#[test]
fn different_emojis_aggregate_separately() {
    let mut store = ReactionsStore::default();
    toggle_reaction(&mut store, "broadcast", "k1", "👍", "owner", "t0");
    toggle_reaction(&mut store, "broadcast", "k1", "🎉", "owner", "t1");
    let agg = aggregate_for_channel(&store, "broadcast");
    assert_eq!(agg.len(), 2);
}

#[test]
fn save_and_load_roundtrips() {
    let dir = tempdir().unwrap();
    let mut store = ReactionsStore::default();
    toggle_reaction(&mut store, "broadcast", "k1", "👍", "owner", "t0");
    save(dir.path(), &store).unwrap();
    let loaded = load(dir.path());
    let agg = aggregate_for_channel(&loaded, "broadcast");
    assert_eq!(agg[0].users, vec!["owner".to_string()]);
}
