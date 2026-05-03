// #1153: paged-history hook stub. Wraps useWebSocket's messages
// state with a bounded array (L most-recent), drop-tail on push when
// the user is near-bottom, and lazy-prepend of older messages on
// scroll-up-near-top via /api/messages?before=<ts>&limit=N. Pinned
// messages exempt from drop-tail.
//
// Stub for the start_story acceptance_test_path validator; real
// implementation lands across the multi-slice plan in #1153.

export {}
