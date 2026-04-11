# Token log schema — #179

The token log is an append-only JSONL file at
`~/.bpmux/tokens/log.jsonl` that records per-turn LLM-call token counts
from any agent harness. It exists so bpmux can surface objective,
harness-sourced cost data without introspecting its own LLM calls (the
"not subjective to you" governance property — dimension 3 of #175).

Each row is one JSON object on one line. Rows are written by
**per-harness adapters** that read the harness's native recording of
its own LLM calls. bpmux reads the log via the
`bpmux::session::tokens::read_token_rows` function.

## Schema

```
{
  "ts": "2026-04-11T22:30:05.000Z",       // RFC3339 string, when the turn happened
  "role": "dev",                           // bpmux role (dev / pm / designer / ryan)
  "harness": "claude-code",                // agent ecosystem name
  "model": "claude-opus-4-6",              // LLM model id
  "input_tokens": 12,                      // u64
  "cache_creation_input_tokens": 0,        // u64
  "cache_read_input_tokens": 8200,         // u64
  "output_tokens": 450,                    // u64
  "tool_call_count": 3,                    // u32 — number of tool_use blocks
  "turn_duration_ms": 8200,                // optional u64
  "transcript_line": 16047,                // optional u64 — debug aid
  "session_id": "abc-123"                  // optional string — debug aid
}
```

Field semantics:

- **ts**: RFC3339 string, NOT an integer. Adapters in any language can
  produce this from their harness's timestamp field. The Rust reader
  parses strings only.
- **role**: the bpmux role the adapter is reporting for. Adapters read
  this from the `BPMUX_ROLE` env var (set by the bpmux plugin's
  session-start hook) or from the role configuration in the agent's
  workspace.
- **harness**: free-form string identifying the agent ecosystem.
  Convention: kebab-case, ASCII, no spaces. Examples: `claude-code`,
  `cursor`, `codex`, `aider`.
- **model**: free-form model id from the LLM call. The adapter pulls
  this from the harness's recording; bpmux does not canonicalize.
- **input_tokens / cache_creation_input_tokens / cache_read_input_tokens / output_tokens**:
  the four Anthropic-shape fields. Providers that don't distinguish
  cache creation vs read (OpenAI, etc.) should set those fields to 0
  and use `input_tokens` for the prompt total.
- **tool_call_count**: number of tool invocations in the turn (for
  Claude Code, the count of `tool_use` blocks in the assistant
  message content). Used as a secondary cost signal.
- **turn_duration_ms / transcript_line / session_id**: optional debug
  aids. `session_id` helps group rows in the view; `transcript_line`
  helps a human trace a row back to its source entry.

## Adapter contract

Each adapter MUST:

1. Read the harness's native recording of its own LLM calls.
   Read-only — adapters never modify the source.
2. Extract per-turn token counts and metadata.
3. Append a row to `~/.bpmux/tokens/log.jsonl` (or the path in
   `BPMUX_TOKENS_DIR/log.jsonl` if that env var is set — tests use
   this to redirect writes to a tempdir).
4. Deduplicate by some harness-stable identifier so repeated hook
   fires don't double-count the same turn. Claude Code's adapter
   uses the assistant-message uuid, stored per-session in
   `~/.bpmux/tokens/.last-uuid-$SESSION_ID`.
5. Exit 0 silently on malformed / missing input. Adapters must not
   fail the harness's hook pipeline.

Each adapter MAY:

- Enrich `model` with harness-specific suffixes if the provider
  doesn't expose a single canonical id.
- Aggregate multiple LLM calls within a turn into one row if doing
  so is cheaper than emitting one row per call. The bpmux reader
  does not care about granularity — it aggregates the rows it sees.
- Emit additional fields beyond the schema above; the Rust `TokenRow`
  struct uses `#[serde(default)]` for optional fields and tolerates
  unknown fields (Rust's serde discards them by default).

## Reference implementations

- **Claude Code**: `claude-plugin/hooks/token-adapter.sh` — bash/jq,
  registered as a `Stop` hook in `hooks.json`. Reads
  `$TRANSCRIPT_PATH` from the hook JSON on stdin, finds the last
  assistant entry with `.message.usage`, writes one row.

Future adapters can live anywhere that can register a
post-turn hook in their target harness. Landing a new adapter does
not require changing bpmux code as long as the row shape matches.

## Verification

The Claude Code adapter is covered end-to-end by
`tests/token_adapter_test.rs`, which feeds a fake transcript to the
real script, then reads the resulting log back via `read_token_rows`
and asserts on the parsed `TokenRow` values. Any new adapter should
add a similar test driven through the real script.

Primitives are unit-tested at
`src/session/tokens.rs::tests_mod` (append/read round-trip, malformed
line skip, missing file, role aggregation).
