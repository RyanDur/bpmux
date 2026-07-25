#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashSet;

use bridge::{DISPATCHED_TOOL_NAMES, build_advertised_tools};

#[test]
fn every_dispatched_tool_is_advertised_in_list_tools() {
    let tools = build_advertised_tools();
    let advertised: HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let dispatched: HashSet<&str> = DISPATCHED_TOOL_NAMES.iter().copied().collect();

    let missing: Vec<&&str> = dispatched.difference(&advertised).collect();
    assert!(
        missing.is_empty(),
        "dispatched tools missing from list_tools advertisement: {missing:?}. \
         Add the corresponding `crate::mcp::tools::*_tool(...)` entry to \
         `build_advertised_tools` in bridge/mod.rs."
    );
}

#[test]
fn every_advertised_tool_has_a_dispatch_arm() {
    let tools = build_advertised_tools();
    let advertised: HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let dispatched: HashSet<&str> = DISPATCHED_TOOL_NAMES.iter().copied().collect();

    let extra: Vec<&&str> = advertised.difference(&dispatched).collect();
    assert!(
        extra.is_empty(),
        "tools advertised but not dispatched (would 'unknown tool' at call-time): {extra:?}. \
         Add the matching match-arm to `call_tool` in bridge/mod.rs and the \
         name to `DISPATCHED_TOOL_NAMES`."
    );
}
