#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

// #1299 Cassiopeia Slice E — regression tests pinning the MCP tool
// registry's no-parallel-shape contract for iteration membership.
// After Slice E retired the 3 iteration_id-axis tools, the registry
// must NOT advertise them. Sister to canonical_tests.rs (substrate-
// axis); this is the bridge-axis gate.

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
