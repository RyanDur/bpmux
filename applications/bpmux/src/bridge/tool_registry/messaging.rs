pub(super) fn messaging_tools() -> Vec<rmcp::model::Tool> {
    vec![
        crate::mcp::tools::send_message_tool(
            "Send a message to the bpmux session. Your message will appear in the shared chat. Do not prefix your name — the session adds the sender automatically.",
        ),
        crate::mcp::tools::share_message_tool(
            "Propagate a message to a wider scope with provenance marking. Two forms: (1) call with `scope` only to re-share your most recent visible chat message (adds `[Shared by <you> from <source> thread]` prefix); (2) call with `scope` and `text` to send fresh text with `[Shared by <you>]` prefix. Scope is `\"all\"` for broadcast or a comma-separated role list (e.g. `\"dev,pm\"`). Privacy gate: explicit-privacy messages (DMs) cannot be re-shared — paraphrase in your own words if they need wider visibility.",
        ),
        crate::mcp::tools::read_messages_tool("Read recent messages from the bpmux session."),
        crate::mcp::tools::subscribe_tool(
            "Subscribe to an event class so it reaches your LLM stream instead of polling. Structured classes (pair with wait_for_event): \"board_invalidated\" (board changes — story moves, labels, accept/retire), \"ci_state_changed\", \"story_state_transition\", \"addressed_to_me\". Chat-relevance classes tune your live chat push-stream: \"chat\" (broadcast chat), \"insight\", \"nudge\". Once you declare ANY chat-relevance class, your live chat push carries ONLY the classes you named (your relevance-set) — declare nothing and a curated default applies (broadcast-chat + insight on, others'-nudges off). Messages addressed to you and owner-sent messages are always pushed and cannot be muted. Suppressed traffic stays in the log, catchable on demand. Returns { subscription_id }.",
        ),
        crate::mcp::tools::unsubscribe_tool(
            "Cancel a subscription created by subscribe. Pass its subscription_id. Returns { unsubscribed }.",
        ),
        crate::mcp::tools::wait_for_event_tool(
            "Block until the next event on a subscription arrives (or timeout_ms elapses). Returns { event_class, payload } on an event, or { timed_out: true } if the window passes with no event — never blocks without recourse.",
        ),
        crate::mcp::tools::search_log_tool(
            "Search the session log by keyword. Returns message clusters with context. Use to find team experiences, decisions, and discussion arcs.",
        ),
    ]
}
