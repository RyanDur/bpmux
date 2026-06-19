#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::send_message::{DeliveredTo, delivery_echo, ensure_owner_included};

#[test]
fn broadcast_when_to_is_none() {
    let echo = delivery_echo(None);
    assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
}

#[test]
fn broadcast_when_to_contains_all() {
    let to = vec!["all".to_string()];
    let echo = delivery_echo(Some(&to));
    assert!(matches!(echo.delivered_to, DeliveredTo::Broadcast(_)));
}

#[test]
fn owner_auto_included_when_absent_from_to() {
    let to = Some(vec!["dev".to_string(), "pm".to_string()]);
    let adjusted = ensure_owner_included(to).expect("scoped");
    assert!(adjusted.iter().any(|r| r == "ryan"));
}

#[test]
fn owner_not_duplicated_when_already_in_to() {
    let to = Some(vec!["ryan".to_string(), "dev".to_string()]);
    let adjusted = ensure_owner_included(to).expect("scoped");
    assert_eq!(adjusted.iter().filter(|r| **r == "ryan").count(), 1);
}

#[test]
fn broadcast_passes_through_unchanged() {
    assert!(ensure_owner_included(None).is_none());
}

#[test]
fn all_keyword_passes_through_unchanged() {
    let to = Some(vec!["all".to_string()]);
    let adjusted = ensure_owner_included(to).expect("scoped");
    assert_eq!(adjusted, vec!["all".to_string()]);
}

#[test]
fn empty_scope_passes_through_unchanged() {
    let to = Some(vec![]);
    let adjusted = ensure_owner_included(to).expect("scoped");
    assert!(adjusted.is_empty());
}

#[test]
fn broadcast_serializes_as_string() -> Result<(), serde_json::Error> {
    let echo = delivery_echo(None);
    let serialized = serde_json::to_value(&echo)?;
    assert_eq!(serialized["delivered_to"], serde_json::json!("broadcast"));
    assert_eq!(serialized["status"], serde_json::json!("sent"));
    Ok(())
}

#[test]
fn echo_has_no_visible_to_owner_field() -> Result<(), serde_json::Error> {
    let echo = delivery_echo(Some(&["dev".to_string()]));
    let serialized = serde_json::to_value(&echo)?;
    assert!(
        serialized.get("visible_to_owner").is_none(),
        "visible_to_owner field must be absent: {serialized}"
    );
    Ok(())
}

#[test]
fn scoped_echo_surfaces_list_with_owner_auto_added() -> Result<(), serde_json::Error> {
    let adjusted = ensure_owner_included(Some(vec!["dev".to_string()])).expect("scoped");
    let echo = delivery_echo(Some(&adjusted));
    let serialized = serde_json::to_value(&echo)?;
    let list = serialized["delivered_to"]
        .as_array()
        .expect("array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();
    assert!(list.contains(&"dev"));
    assert!(list.contains(&"ryan"));
    Ok(())
}
