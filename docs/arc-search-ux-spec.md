# Arc Search UX Spec

*Designer spec — 2026-04-08. How agents and humans interact with log search.*

## Two Interfaces, One Search

Search serves two users with different needs:

1. **Agents** — query via bridge tools programmatically. Need structured results they can process.
2. **Ryan** — types in the TUI. Needs readable results he can scroll through.

Both use the same search backend. The difference is rendering.

---

## Agent Interface (Bridge Tools)

### `search_log(query, filters)`

**Parameters:**
- `query` (string) — natural language or keyword. "What has this team learned about role boundaries?"
- `filters` (optional):
  - `sender` — filter by sender name (e.g., "ryan")
  - `after` / `before` — time range (ISO 8601)
  - `channel` — filter by channel name
  - `type` — "correction" | "discussion" | "decision" | "all"

**Returns:**
```json
{
  "results": [
    {
      "messages": [
        {"from": "ryan:human", "text": "designer, if you want a feature...", "ts": "..."},
        {"from": "designer:claude-code", "text": "You're right...", "ts": "..."}
      ],
      "relevance": 0.85,
      "summary": "Owner corrected designer about routing features through PM"
    }
  ],
  "total": 3
}
```

Results are message sequences, not individual messages. Each result is a cluster of related messages — the beginning of an arc.

### Usage by a cold-starting agent

```
1. Agent joins session
2. Calls search_log("what are the team norms?", {sender: "ryan", type: "correction"})
3. Gets back Ryan's correction sequences
4. Reads them and builds understanding
```

No arc database. No manual capture. Just structured search over the log.

---

## Human Interface (TUI)

### `/search <query>`

Ryan types `/search role boundaries` in the TUI.

**Result rendering:**
- Results appear as system messages in the chat area
- Each result shows a message cluster with context
- Highlighted search terms
- Timestamp and sender for each message
- "N more results" indicator if there are many matches

**Layout:**
```
─── Search: "role boundaries" (3 results) ───

[1] 2026-04-06 03:15
  ryan: designer, if you want a feature you need to
        go through the pm
  designer: You're right. Sorry — PM, the recommendation
            is to remove the channel count...

[2] 2026-04-07 22:35
  ryan: dev is in charge of logic
  designer: You're right — I prescribed implementation
            again...

[3] 2026-04-08 11:05
  ryan: design, go through pm
  designer: You're right. Sorry — PM, the recommendation...

─── End of results ───
```

Results are scrollable like any long message. Enter expands a result to show more context (surrounding messages).

### `/search` with no query

Shows recent notable sequences — owner corrections, story acceptances, retro outcomes. The "what should I know?" default.

---

## Design Decisions

**Why message clusters, not individual messages:** A single message lacks context. "designer, if you want a feature you need to go through the pm" means nothing without the preceding message that triggered the correction. Clusters preserve the reasoning path — trigger + response + resolution.

**Why no separate search view:** Search results render in the same chat area as messages. No mode switch, no separate panel. Keeps the TUI simple — one primary view (chat), everything renders there.

**Why `/search` not a sidebar feature:** Search is an action, not a navigation state. You search, read results, and go back to chatting. The sidebar is for persistent navigation (channels). Search is transient.

**Cluster detection:** Group messages by temporal proximity (messages within N minutes of each other) and topic coherence (same sender thread). The simplest clustering: find the query match, include 3 messages before and 3 after. More sophisticated: expand the cluster until the topic shifts (different sender, different channel, gap > 5 minutes).

---

## Open Questions

1. Should search results persist in the chat or disappear when you scroll away?
2. Should agents be able to search each other's private channels, or only broadcasts?
3. How does search handle the 80-char width — long results need word wrapping and truncation.
4. Should there be a "search history" so Ryan can re-run previous queries?
