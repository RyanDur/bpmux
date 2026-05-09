use super::iteration_markers::{
    CONSTELLATION_THEME_POOL, IterationMarker, IterationMarkersAction, IterationMarkersStore,
};
use super::stores::StoreReduceOutcome;

fn marker(id: u64, theme: &str, position: u32) -> IterationMarker {
    IterationMarker {
        id,
        theme_name: theme.to_string(),
        position,
        created_at: "2026-05-09T19:00:00Z".to_string(),
    }
}

#[test]
fn list_returns_markers_sorted_by_position() {
    let store = IterationMarkersStore::new();
    store.apply(&IterationMarkersAction::MarkerAdded(marker(2, "Lyra", 1)));
    store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Orion", 0)));
    let listed = store.list();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].theme_name, "Orion");
    assert_eq!(listed[1].theme_name, "Lyra");
}

#[test]
fn marker_added_returns_changed_outcome() {
    let store = IterationMarkersStore::new();
    let outcome = store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Orion", 0)));
    assert_eq!(outcome, StoreReduceOutcome::Changed);
}

#[test]
fn duplicate_marker_id_returns_unchanged() {
    let store = IterationMarkersStore::new();
    store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Orion", 0)));
    let outcome = store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Lyra", 1)));
    assert_eq!(outcome, StoreReduceOutcome::Unchanged);
    assert_eq!(store.list().len(), 1);
}

#[test]
fn marker_removed_drops_entry() {
    let store = IterationMarkersStore::new();
    store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Orion", 0)));
    let outcome = store.apply(&IterationMarkersAction::MarkerRemoved { id: 1 });
    assert_eq!(outcome, StoreReduceOutcome::Changed);
    assert!(store.list().is_empty());
}

#[test]
fn marker_removed_unknown_id_returns_unchanged() {
    let store = IterationMarkersStore::new();
    let outcome = store.apply(&IterationMarkersAction::MarkerRemoved { id: 99 });
    assert_eq!(outcome, StoreReduceOutcome::Unchanged);
}

#[test]
fn allocate_id_returns_monotonic_ids() {
    let store = IterationMarkersStore::new();
    let a = store.allocate_id();
    let b = store.allocate_id();
    let c = store.allocate_id();
    assert_eq!(a, 0);
    assert_eq!(b, 1);
    assert_eq!(c, 2);
}

#[test]
fn next_position_starts_at_zero_then_increments_above_max() {
    let store = IterationMarkersStore::new();
    assert_eq!(store.next_position(), 0);
    store.apply(&IterationMarkersAction::MarkerAdded(marker(1, "Orion", 5)));
    assert_eq!(store.next_position(), 6);
    store.apply(&IterationMarkersAction::MarkerAdded(marker(2, "Lyra", 2)));
    assert_eq!(store.next_position(), 6);
}

#[test]
fn next_theme_name_picks_first_unused_constellation() {
    let store = IterationMarkersStore::new();
    let first = store.next_theme_name();
    assert_eq!(first, CONSTELLATION_THEME_POOL[0]);
    store.apply(&IterationMarkersAction::MarkerAdded(marker(1, &first, 0)));
    let second = store.next_theme_name();
    assert_eq!(second, CONSTELLATION_THEME_POOL[1]);
    assert_ne!(second, first);
}

#[test]
fn next_theme_name_falls_back_to_numbered_when_pool_exhausted() {
    let store = IterationMarkersStore::new();
    for (idx, theme) in CONSTELLATION_THEME_POOL.iter().enumerate() {
        store.apply(&IterationMarkersAction::MarkerAdded(marker(
            idx as u64, theme, idx as u32,
        )));
    }
    let next = store.next_theme_name();
    assert!(next.starts_with("Marker "));
}

#[test]
fn theme_pool_contains_canonical_constellations() {
    let pool: std::collections::HashSet<&&str> = CONSTELLATION_THEME_POOL.iter().collect();
    assert!(pool.contains(&"Orion"));
    assert!(pool.contains(&"Lyra"));
    assert!(pool.contains(&"Cassiopeia"));
    assert!(pool.contains(&"Cygnus"));
    assert!(pool.contains(&"Andromeda"));
    assert!(CONSTELLATION_THEME_POOL.len() >= 30);
}
