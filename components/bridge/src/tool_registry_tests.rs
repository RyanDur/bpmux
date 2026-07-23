#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use super::{DISPATCHED_TOOL_NAMES, build_advertised_tools};

#[test]
fn tool_registry_does_not_advertise_retired_iteration_id_axis_tools() {
    let advertised: Vec<String> = build_advertised_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    for retired in &[
        "add_stories_to_iteration",
        "remove_stories_from_iteration",
        "list_iteration_children",
    ] {
        assert!(
            !advertised.iter().any(|n| n == retired),
            "{retired} retired #1299 Slice E — must not be advertised; advertised={:?}",
            advertised
        );
    }
}

#[test]
fn dispatch_table_does_not_include_retired_iteration_id_axis_tools() {
    for retired in &[
        "add_stories_to_iteration",
        "remove_stories_from_iteration",
        "list_iteration_children",
    ] {
        assert!(
            !DISPATCHED_TOOL_NAMES.iter().any(|n| n == retired),
            "{retired} dispatch entry retired #1299 Slice E; got={:?}",
            DISPATCHED_TOOL_NAMES
        );
    }
}

#[test]
fn retired_set_status_advertisement_drops_the_removed_available_presence_state() {
    let tools = build_advertised_tools();
    let set_status = tools
        .iter()
        .find(|t| t.name.as_ref() == "set_status")
        .expect("set_status stays advertised — parity with its dispatch redirect (#1404)");
    let serialized = serde_json::to_string(set_status)
        .expect("advertised tool serializes")
        .to_ascii_lowercase();
    assert!(
        !serialized.contains("available"),
        "set_status still advertises the removed `available` presence state (#1750 vestige); tool={serialized}"
    );
    assert!(
        serialized.contains("retire") || serialized.contains("hook"),
        "the retired set_status tool must advertise its hook-managed reality, not a live status-setter; tool={serialized}"
    );
}

#[test]
fn positional_substrate_tools_remain_advertised() {
    let advertised: Vec<String> = build_advertised_tools()
        .iter()
        .map(|t| t.name.to_string())
        .collect();
    for kept in &["create_iteration", "close_iteration"] {
        assert!(
            advertised.iter().any(|n| n == kept),
            "{kept} must remain advertised after Slice E (positional substrate); advertised={:?}",
            advertised
        );
    }
}

#[test]
fn the_advertised_send_message_description_warns_against_name_prefixing() {
    let warning = build_advertised_tools()
        .into_iter()
        .find(|tool| tool.name == "send_message")
        .and_then(|tool| tool.description.map(|d| d.to_string()))
        .unwrap_or_default();
    assert!(
        warning.contains("Do not prefix your name"),
        "the REAL advertised send_message tool carries the self-prefix warning; got: {warning}"
    );
}
