use super::filter_matches;
use serde_json::{Map, Value, json};

fn criteria(pairs: &[(&str, Value)]) -> Option<Map<String, Value>> {
    let mut map = Map::new();
    for (key, value) in pairs {
        map.insert((*key).to_string(), value.clone());
    }
    Some(map)
}

#[test]
fn none_filter_matches_any_payload() {
    assert!(filter_matches(
        &None,
        &json!({ "head_sha": "A", "run_id": 42 })
    ));
    assert!(filter_matches(&None, &json!({})));
}

#[test]
fn submap_filter_matches_payload_superset() {
    let filter = criteria(&[("head_sha", json!("A"))]);
    assert!(filter_matches(
        &filter,
        &json!({ "head_sha": "A", "run_id": 42 })
    ));
}

#[test]
fn filter_does_not_match_on_value_mismatch() {
    let filter = criteria(&[("head_sha", json!("A"))]);
    assert!(!filter_matches(&filter, &json!({ "head_sha": "B" })));
}

#[test]
fn filter_on_absent_key_does_not_match() {
    let filter = criteria(&[("head_sha", json!("A"))]);
    assert!(!filter_matches(&filter, &json!({ "run_id": 42 })));
}
