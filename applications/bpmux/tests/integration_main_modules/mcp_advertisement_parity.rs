//! Parity test for #531: every tool dispatched by `BridgeServer::call_tool`
//! must also be advertised by `list_tools`, and vice-versa.
//!
//! #530 shipped `edit_story`'s handler + dispatch arm + RPC route but
//! never registered it in the MCP advertisement layer — claude-code's
//! MCP client never saw the tool. PM/dev/designer all blocked despite
//! a fully-functional handler. This test catches the gap structurally
//! at unit-test time: future tool additions that miss either side of
//! the registration cascade fail here before delivery.

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
